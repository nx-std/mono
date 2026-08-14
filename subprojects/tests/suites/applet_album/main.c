// Album applet (`photoViewer` library applet) checks.
//
// Interactive by nature: each `albumLaShow*` launches a separate process and
// blocks until someone leaves the Album, so these cannot run unattended and
// live outside the `nx-tests` suite.
//
// The same binary links with or without `use_nx_service_applet_album`. With the
// option off the calls resolve to libnx and show what the stock implementation
// does; with it on they resolve to `__nx_rt_hbapp__libnx_album_la_*`. Running both
// is the comparison worth making.

#include <stdio.h>
#include <switch.h>

// Shows only the files this application created. The filter button is disabled,
// which is the visible difference from the next case.
static void show_album_files(void) {
    Result rc = albumLaShowAlbumFiles();
    printf("  albumLaShowAlbumFiles()             -> 0x%x\n", rc);
    consoleUpdate(NULL);
}

// Shows every file on the console, with filtering allowed.
static void show_all_album_files(void) {
    Result rc = albumLaShowAllAlbumFiles();
    printf("  albumLaShowAllAlbumFiles()          -> 0x%x\n", rc);
    consoleUpdate(NULL);
}

// Same set as above, but launched the way the HOME menu does it. The audible
// difference is the point: this is the only one that plays the startup sound,
// which is what proves `play_startup_sound` reached the common arguments.
static void show_all_album_files_for_home_menu(void) {
    Result rc = albumLaShowAllAlbumFilesForHomeMenu();
    printf("  albumLaShowAllAlbumFilesForHomeMenu -> 0x%x\n", rc);
    consoleUpdate(NULL);
}

int main(int argc, char *argv[]) {
    consoleInit(NULL);

    PadState pad;
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);
    padInitializeDefault(&pad);

    printf("nx-tests-applet-album\n\n");
    printf("A     show album files (this application only)\n");
    printf("X     show all album files\n");
    printf("Y     show all album files, as the HOME menu does\n");
    printf("PLUS  exit\n\n");
    consoleUpdate(NULL);

    while (appletMainLoop()) {
        padUpdate(&pad);
        u64 down = padGetButtonsDown(&pad);

        if (down & HidNpadButton_Plus) {
            break;
        }
        if (down & HidNpadButton_A) {
            show_album_files();
        }
        if (down & HidNpadButton_X) {
            show_all_album_files();
        }
        if (down & HidNpadButton_Y) {
            show_all_album_files_for_home_menu();
        }

        consoleUpdate(NULL);
    }

    consoleExit(NULL);
    return 0;
}
