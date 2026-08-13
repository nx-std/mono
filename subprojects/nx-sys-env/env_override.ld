/* Static linker script for environment symbol redirection */
/* Redirects newlib functions to nx-sys-env __nx_sys_env__* implementations */

/* libc (newlib) */
EXTERN(__nx_sys_env__newlib_getenv)
EXTERN(__nx_sys_env__newlib_getenv_r)
EXTERN(__nx_sys_env__newlib_setenv)
EXTERN(__nx_sys_env__newlib_setenv_r)
EXTERN(__nx_sys_env__newlib_unsetenv)
EXTERN(__nx_sys_env__newlib_unsetenv_r)
EXTERN(__nx_sys_env__newlib_putenv)
EXTERN(__nx_sys_env__newlib_putenv_r)
EXTERN(__nx_sys_env__newlib_environ)

getenv     = __nx_sys_env__newlib_getenv;
_getenv_r  = __nx_sys_env__newlib_getenv_r;
setenv     = __nx_sys_env__newlib_setenv;
_setenv_r  = __nx_sys_env__newlib_setenv_r;
unsetenv   = __nx_sys_env__newlib_unsetenv;
_unsetenv_r = __nx_sys_env__newlib_unsetenv_r;
putenv     = __nx_sys_env__newlib_putenv;
_putenv_r  = __nx_sys_env__newlib_putenv_r;

/* The environment block itself. Redirecting the readers alone would leave the C
   library's own array in place; redirecting this makes the Rust store the one
   environment, including for `_findenv_r`, which walks it directly. */
environ    = __nx_sys_env__newlib_environ;
