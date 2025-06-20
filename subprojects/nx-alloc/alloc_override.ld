/* Static linker script for allocation function symbols redirection */
/* Redirects libnx functions to nx-alloc __nx_alloc_* implementations */

/* init.c */
EXTERN(__nx_alloc_init_heap)

__libnx_initheap = __nx_alloc_init_heap;

/* libc (newlib) */
EXTERN(__nx_alloc_newlib_malloc_r)
EXTERN(__nx_alloc_newlib_calloc_r)
EXTERN(__nx_alloc_newlib_realloc_r)
EXTERN(__nx_alloc_newlib_memalign_r)
EXTERN(__nx_alloc_newlib_free_r)

_malloc_r   = __nx_alloc_newlib_malloc_r;
_calloc_r   = __nx_alloc_newlib_calloc_r;
_realloc_r  = __nx_alloc_newlib_realloc_r;
_memalign_r = __nx_alloc_newlib_memalign_r;
_free_r     = __nx_alloc_newlib_free_r;
