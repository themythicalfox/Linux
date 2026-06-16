// ArchonSync "taskbar" layout — Windows-familiar: one bottom panel with a
// start-style launcher, a task manager (window list), the system tray and a
// clock. Applied via plasmashell evaluateScript.

var existing = panels();
for (var i = 0; i < existing.length; i++) {
    existing[i].remove();
}

var bar = new Panel;
bar.location = "bottom";
bar.height = 44;

// Start menu (Kickoff), pinned tasks + running windows, spacer, tray, clock.
bar.addWidget("org.kde.plasma.kickoff");
bar.addWidget("org.kde.plasma.icontasks");
bar.addWidget("org.kde.plasma.marginsseparator");
bar.addWidget("org.kde.plasma.systemtray");
bar.addWidget("org.kde.plasma.digitalclock");
bar.addWidget("org.kde.plasma.showdesktop");

// Wallpaper on every desktop.
for (var j = 0; j < desktops().length; j++) {
    var d = desktops()[j];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", "file:///usr/share/wallpapers/ArchonSync/contents/images/3840x2160.png");
    d.writeConfig("FillMode", 2);
}
