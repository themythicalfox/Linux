/*
 * ArchonSync Wheel — a dot at the screen edge that expands into a
 * scrollable radial launcher.
 *
 * Hovering the dot opens the wheel. Scrolling rotates the wheel one
 * entry at a time; the selected entry sits at the top and is shown in
 * the center. Clicking the center (or any entry) launches it.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as P5Support

PlasmoidItem {
    id: root

    readonly property color accent: "#ff7a1a"
    readonly property color surface: "#141418"
    readonly property color surfaceLine: "#2a2a32"
    readonly property color textColor: "#e8e8ec"
    readonly property color textDim: "#8a8a96"

    property var entries: {
        try {
            return JSON.parse(plasmoid.configuration.entries)
        } catch (e) {
            return []
        }
    }
    property int selected: 0

    preferredRepresentation: compactRepresentation
    switchWidth: Kirigami.Units.gridUnit * 100   // never switch to full in-panel
    switchHeight: Kirigami.Units.gridUnit * 100

    P5Support.DataSource {
        id: runner
        engine: "executable"
        onNewData: sourceName => disconnectSource(sourceName)
    }

    function launch(index) {
        const entry = entries[index]
        if (entry && entry.cmd) {
            runner.connectSource(entry.cmd)
        }
        root.expanded = false
    }

    function rotate(steps) {
        const n = entries.length
        if (n > 0) {
            selected = ((selected + steps) % n + n) % n
        }
    }

    compactRepresentation: Item {
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            onContainsMouseChanged: if (containsMouse) root.expanded = true
            onClicked: root.expanded = !root.expanded
        }

        Rectangle {
            id: ring
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height) * 0.85
            height: width
            radius: width / 2
            color: "transparent"
            border.color: Qt.alpha(root.accent, root.expanded ? 0.9 : 0.35)
            border.width: 1
        }

        Rectangle {
            id: dot
            anchors.centerIn: parent
            width: ring.width * 0.5
            height: width
            radius: width / 2
            color: root.accent
            scale: root.expanded ? 1.25 : 1.0
            Behavior on scale { NumberAnimation { duration: 120 } }
        }
    }

    fullRepresentation: Item {
        id: wheel

        readonly property int size: plasmoid.configuration.wheelSize
        readonly property real ringRadius: size / 2 - Kirigami.Units.gridUnit * 2

        Layout.preferredWidth: size
        Layout.preferredHeight: size
        Layout.minimumWidth: size
        Layout.minimumHeight: size

        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => root.rotate(event.angleDelta.y < 0 ? 1 : -1)
        }

        // Wheel backdrop
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: root.surface
            border.color: root.surfaceLine
            border.width: 1
        }

        // Selection marker at the top of the ring
        Rectangle {
            width: 6; height: 6; radius: 3
            color: root.accent
            anchors.horizontalCenter: parent.horizontalCenter
            y: Kirigami.Units.smallSpacing
        }

        // Entries arranged around the ring; the wheel rotates so the
        // selected entry is always at the top.
        Repeater {
            model: root.entries
            delegate: Item {
                readonly property real angle:
                    ((index - root.selected) / root.entries.length) * 2 * Math.PI - Math.PI / 2
                readonly property bool isSelected: index === root.selected

                width: Kirigami.Units.iconSizes.medium + Kirigami.Units.smallSpacing * 2
                height: width
                x: wheel.width / 2 + Math.cos(angle) * wheel.ringRadius - width / 2
                y: wheel.height / 2 + Math.sin(angle) * wheel.ringRadius - height / 2

                Behavior on x { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }
                Behavior on y { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }

                Rectangle {
                    anchors.fill: parent
                    radius: width / 2
                    color: isSelected ? Qt.alpha(root.accent, 0.18) : "transparent"
                    border.color: isSelected ? root.accent : "transparent"
                    border.width: 1
                }

                Kirigami.Icon {
                    anchors.centerIn: parent
                    width: Kirigami.Units.iconSizes.medium * (isSelected ? 1.0 : 0.75)
                    height: width
                    source: modelData.icon
                    opacity: isSelected ? 1.0 : 0.55
                    Behavior on width { NumberAnimation { duration: 140 } }
                    Behavior on opacity { NumberAnimation { duration: 140 } }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        if (isSelected) {
                            root.launch(index)
                        } else {
                            root.selected = index
                        }
                    }
                }
            }
        }

        // Center hub: selected entry name, click to launch
        Rectangle {
            id: hub
            anchors.centerIn: parent
            width: wheel.size * 0.42
            height: width
            radius: width / 2
            color: "#0c0c0f"
            border.color: hubMouse.containsMouse ? root.accent : root.surfaceLine
            border.width: 1

            ColumnLayout {
                anchors.centerIn: parent
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    Layout.alignment: Qt.AlignHCenter
                    width: Kirigami.Units.iconSizes.large
                    height: width
                    source: root.entries[root.selected] ? root.entries[root.selected].icon : "applications-all"
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: root.entries[root.selected] ? root.entries[root.selected].name : ""
                    color: root.textColor
                    font.pointSize: 10
                    font.letterSpacing: 1.5
                    font.capitalization: Font.AllUppercase
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "scroll · click"
                    color: root.textDim
                    font.pointSize: 7
                    font.letterSpacing: 1
                }
            }

            MouseArea {
                id: hubMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: root.launch(root.selected)
            }
        }
    }
}
