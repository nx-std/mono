/*
 * Human Input Device (HID) API
 * Rust: ffi/libnx/hid.rs
 * libnx: services/hid.c
 */
EXTERN(__nx_rt_nro__libnx_hid_initialize);
EXTERN(__nx_rt_nro__libnx_hid_exit);
EXTERN(__nx_rt_nro__libnx_hid_get_sharedmem_addr);
EXTERN(__nx_rt_nro__libnx_hid_initialize_npad);
EXTERN(__nx_rt_nro__libnx_hid_set_supported_npad_style_set);
EXTERN(__nx_rt_nro__libnx_hid_set_supported_npad_id_type);
EXTERN(__nx_rt_nro__libnx_hid_initialize_touch_screen);
EXTERN(__nx_rt_nro__libnx_hid_initialize_mouse);
EXTERN(__nx_rt_nro__libnx_hid_initialize_keyboard);
EXTERN(__nx_rt_nro__libnx_hid_initialize_gesture);

hidInitialize = __nx_rt_nro__libnx_hid_initialize;
hidExit = __nx_rt_nro__libnx_hid_exit;
hidGetSharedmemAddr = __nx_rt_nro__libnx_hid_get_sharedmem_addr;
hidInitializeNpad = __nx_rt_nro__libnx_hid_initialize_npad;
hidSetSupportedNpadStyleSet = __nx_rt_nro__libnx_hid_set_supported_npad_style_set;
hidSetSupportedNpadIdType = __nx_rt_nro__libnx_hid_set_supported_npad_id_type;
hidInitializeTouchScreen = __nx_rt_nro__libnx_hid_initialize_touch_screen;
hidInitializeMouse = __nx_rt_nro__libnx_hid_initialize_mouse;
hidInitializeKeyboard = __nx_rt_nro__libnx_hid_initialize_keyboard;
hidInitializeGesture = __nx_rt_nro__libnx_hid_initialize_gesture;
