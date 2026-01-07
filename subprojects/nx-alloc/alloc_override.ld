/* Static linker script for allocation function symbols redirection */
/* Redirects libnx functions to nx-alloc __nx_alloc__* implementations */

/* libc (newlib) */
EXTERN(__nx_alloc__newlib_malloc_r)
EXTERN(__nx_alloc__newlib_calloc_r)
EXTERN(__nx_alloc__newlib_realloc_r)
EXTERN(__nx_alloc__newlib_memalign_r)
EXTERN(__nx_alloc__newlib_free_r)

_malloc_r   = __nx_alloc__newlib_malloc_r;
_calloc_r   = __nx_alloc__newlib_calloc_r;
_realloc_r  = __nx_alloc__newlib_realloc_r;
_memalign_r = __nx_alloc__newlib_memalign_r;
_free_r     = __nx_alloc__newlib_free_r;
