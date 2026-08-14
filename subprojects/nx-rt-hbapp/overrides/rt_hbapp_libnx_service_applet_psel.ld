/*
 * Player-select applet (`playerSelect` library applet) API
 * Rust: ffi/libnx/psel.rs
 * libnx: applets/psel.c
 *
 * Covers every `psel*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `psel*` function left to
 * libnx has a session to use. All twelve exported entry points are ported; the
 * three `pselUiSet*` helpers are `static inline` in psel.h and export no symbol.
 */

/* UI settings */
EXTERN(__nx_rt_hbapp__libnx_psel_ui_create);
EXTERN(__nx_rt_hbapp__libnx_psel_ui_add_user);
EXTERN(__nx_rt_hbapp__libnx_psel_ui_show);

/* User selector screens */
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_selector_for_system);
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_selector_for_launcher);
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_selector);

/* User creation screens */
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_creator);
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_creator_for_starter);

/* User editing screens */
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_icon_editor);
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_nickname_editor);

/* Nintendo Account screens */
EXTERN(__nx_rt_hbapp__libnx_psel_show_nintendo_account_nnid_linker);
EXTERN(__nx_rt_hbapp__libnx_psel_show_user_qualification_promoter);

/* UI settings */
pselUiCreate = __nx_rt_hbapp__libnx_psel_ui_create;
pselUiAddUser = __nx_rt_hbapp__libnx_psel_ui_add_user;
pselUiShow = __nx_rt_hbapp__libnx_psel_ui_show;

/* User selector screens */
pselShowUserSelectorForSystem = __nx_rt_hbapp__libnx_psel_show_user_selector_for_system;
pselShowUserSelectorForLauncher = __nx_rt_hbapp__libnx_psel_show_user_selector_for_launcher;
pselShowUserSelector = __nx_rt_hbapp__libnx_psel_show_user_selector;

/* User creation screens */
pselShowUserCreator = __nx_rt_hbapp__libnx_psel_show_user_creator;
pselShowUserCreatorForStarter = __nx_rt_hbapp__libnx_psel_show_user_creator_for_starter;

/* User editing screens */
pselShowUserIconEditor = __nx_rt_hbapp__libnx_psel_show_user_icon_editor;
pselShowUserNicknameEditor = __nx_rt_hbapp__libnx_psel_show_user_nickname_editor;

/* Nintendo Account screens */
pselShowNintendoAccountNnidLinker = __nx_rt_hbapp__libnx_psel_show_nintendo_account_nnid_linker;
pselShowUserQualificationPromoter = __nx_rt_hbapp__libnx_psel_show_user_qualification_promoter;
