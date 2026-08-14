// Error applet (`error` library applet) checks.
//
// Interactive by nature: `errorApplicationShow` launches a separate process and
// blocks until someone dismisses the dialog, so these cannot run unattended and
// live outside the `nx-tests` suite.
//
// The same binary links with or without `use_nx_service_applet_err`. With the
// option off the calls resolve to libnx and show what the stock implementation
// does; with it on they resolve to `__nx_rt_hbapp__libnx_error_*`. Running both
// is the comparison worth making.

#include <stdio.h>
#include <switch.h>

// Text the dialog should display. Deliberately distinctive: the whole point of
// the applet is that this exact string reaches the screen.
#define DIALOG_MESSAGE "nx-std: application error dialog"
#define DETAILS_MESSAGE                                                        \
    "This text came from errorApplicationShow and is displayed behind the "    \
    "Details button. Seeing it means the argument storage crossed to the "     \
    "applet with its layout intact."

static void show_application_error(void) {
    ErrorApplicationConfig cfg;

    Result rc = errorApplicationCreate(&cfg, DIALOG_MESSAGE, DETAILS_MESSAGE);
    printf("  errorApplicationCreate() -> 0x%x\n", rc);
    consoleUpdate(NULL);

    if (R_FAILED(rc)) {
        return;
    }

    rc = errorApplicationShow(&cfg);
    printf("  errorApplicationShow()   -> 0x%x\n", rc);
    consoleUpdate(NULL);
}

// Exercises a command that is aliased but not implemented. With the override
// active this must panic naming `errorResultShow`, which is what proves the
// linker aliases actually bind; without it, libnx shows its own dialog.
static void show_unimplemented(void) {
    printf("  calling errorResultShow (expect a panic when overridden)...\n");
    consoleUpdate(NULL);

    // Any result value does; the applet only renders it.
    Result rc = errorResultShow(MAKERESULT(Module_Libnx, LibnxError_NotFound), true, NULL);
    printf("  errorResultShow()        -> 0x%x (no panic: not overridden)\n", rc);
    consoleUpdate(NULL);
}

int main(void) {
    consoleInit(NULL);

    PadState pad;
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);
    padInitializeDefault(&pad);

    printf("nx-tests-applet-err\n");
    printf("-------------------\n\n");
    printf("  [A]  show the application error dialog\n");
    printf("  [Y]  call errorResultShow (unimplemented)\n");
    printf("  [+]  exit\n\n");
    consoleUpdate(NULL);

    while (appletMainLoop()) {
        padUpdate(&pad);
        u64 down = padGetButtonsDown(&pad);

        if (down & HidNpadButton_Plus) {
            break;
        }

        if (down & HidNpadButton_A) {
            printf("[A] application error\n");
            consoleUpdate(NULL);
            show_application_error();
        }

        if (down & HidNpadButton_Y) {
            printf("[Y] unimplemented command\n");
            consoleUpdate(NULL);
            show_unimplemented();
        }

        consoleUpdate(NULL);
    }

    consoleExit(NULL);
    return 0;
}
