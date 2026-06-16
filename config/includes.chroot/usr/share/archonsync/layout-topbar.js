// ArchonSync "topbar" layout — macOS-familiar: a slim top bar with a launcher,
// the active window's title space, system tray and clock; a centered floating
// dock of favourite apps along the bottom. Applied via evaluateScript.

var existing = panels();
for (var i = 0; i < existing.length; i++) {
    existing[i].remove();
}

// Slim top menu bar.
var top = new Panel;
top.location = "top";
top.height = 30;
top.addWidget("org.kde.plasma.kickoff");
top.addWidget("org.kde.plasma.appmenu");
top.addWidget("org.kde.plasma.panelspacer");
top.addWidget("org.kde.plasma.systemtray");
top.addWidget("org.kde.plasma.digitalclock");

// Floating bottom dock of pinned apps.
var dock = new Panel;
dock.location = "bottom";
dock.height = 56;
dock.hiding = "dodgewindows";
dock.addWidget("org.kde.plasma.icontasks");

for (var j = 0; j < desktops().length; j++) {
    var d = desktops()[j];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", "file:///usr/share/wallpapers/ArchonSync/contents/images/3840x2160.png");
    d.writeConfig("FillMode", 2);
}
