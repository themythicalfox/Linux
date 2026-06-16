// ArchonSync desktop layout, applied via plasmashell evaluateScript.
// Replaces the default panels with a thin left bar holding the Wheel
// launcher, plus a minimal top bar with clock and system tray.

var existing = panels();
for (var i = 0; i < existing.length; i++) {
    existing[i].remove();
}

// Minimal top bar: clock + system tray only.
var top = new Panel;
top.location = "top";
top.height = 34;
top.addWidget("org.kde.plasma.digitalclock");
top.addWidget("org.kde.plasma.systemtray");

// Left edge: the dot that expands into the Wheel launcher.
var left = new Panel;
left.location = "left";
left.height = 30;        // thickness of a vertical panel
left.addWidget("org.archonsync.wheel");

// Wallpaper on every desktop containment.
var screens = desktopsForActivity(currentActivity());
for (var j = 0; j < desktops().length; j++) {
    var d = desktops()[j];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", "file:///usr/share/wallpapers/ArchonSync/contents/images/3840x2160.png");
    d.writeConfig("FillMode", 2);
}
