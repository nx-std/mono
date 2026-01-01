/* Static linker script for sync function symbols redirection */
/* Redirects libnx functions to nx-sys-sync __nx_sys_sync_* implementations */

/* Barrier */
EXTERN(__nx_sys_sync__barrier_init);
EXTERN(__nx_sys_sync__barrier_wait);

barrierInit = __nx_sys_sync__barrier_init;
barrierWait = __nx_sys_sync__barrier_wait;

/* CondVar */
EXTERN(__nx_sys_sync__condvar_init);
EXTERN(__nx_sys_sync__condvar_wait_timeout);
EXTERN(__nx_sys_sync__condvar_wait);
EXTERN(__nx_sys_sync__condvar_wake);
EXTERN(__nx_sys_sync__condvar_wake_one);
EXTERN(__nx_sys_sync__condvar_wake_all);

condvarInit = __nx_sys_sync__condvar_init;
condvarWaitTimeout = __nx_sys_sync__condvar_wait_timeout;
condvarWait = __nx_sys_sync__condvar_wait;
condvarWake = __nx_sys_sync__condvar_wake;
condvarWakeOne = __nx_sys_sync__condvar_wake_one;
condvarWakeAll = __nx_sys_sync__condvar_wake_all;

/* Mutex */
EXTERN(__nx_sys_sync__mutex_init);
EXTERN(__nx_sys_sync__mutex_lock);
EXTERN(__nx_sys_sync__mutex_try_lock);
EXTERN(__nx_sys_sync__mutex_unlock);
EXTERN(__nx_sys_sync__mutex_is_locked_by_current_thread);

mutexInit = __nx_sys_sync__mutex_init;
mutexLock = __nx_sys_sync__mutex_lock;
mutexTryLock = __nx_sys_sync__mutex_try_lock;
mutexUnlock = __nx_sys_sync__mutex_unlock;
mutexIsLockedByCurrentThread = __nx_sys_sync__mutex_is_locked_by_current_thread;

/* RMutex */
EXTERN(__nx_sys_sync__remutex_init);
EXTERN(__nx_sys_sync__remutex_lock);
EXTERN(__nx_sys_sync__remutex_try_lock);
EXTERN(__nx_sys_sync__remutex_unlock);

rmutexInit = __nx_sys_sync__remutex_init;
rmutexLock = __nx_sys_sync__remutex_lock;
rmutexTryLock = __nx_sys_sync__remutex_try_lock;
rmutexUnlock = __nx_sys_sync__remutex_unlock;

/* RwLock */
EXTERN(__nx_sys_sync__rwlock_init);
EXTERN(__nx_sys_sync__rwlock_read_lock);
EXTERN(__nx_sys_sync__rwlock_try_read_lock);
EXTERN(__nx_sys_sync__rwlock_read_unlock);
EXTERN(__nx_sys_sync__rwlock_write_lock);
EXTERN(__nx_sys_sync__rwlock_try_write_lock);
EXTERN(__nx_sys_sync__rwlock_write_unlock);
EXTERN(__nx_sys_sync__rwlock_is_write_lock_held_by_current_thread);
EXTERN(__nx_sys_sync__rwlock_is_owned_by_current_thread);

rwlockInit = __nx_sys_sync__rwlock_init;
rwlockReadLock = __nx_sys_sync__rwlock_read_lock;
rwlockTryReadLock = __nx_sys_sync__rwlock_try_read_lock;
rwlockReadUnlock = __nx_sys_sync__rwlock_read_unlock;
rwlockWriteLock = __nx_sys_sync__rwlock_write_lock;
rwlockTryWriteLock = __nx_sys_sync__rwlock_try_write_lock;
rwlockWriteUnlock = __nx_sys_sync__rwlock_write_unlock;
rwlockIsWriteLockHeldByCurrentThread = __nx_sys_sync__rwlock_is_write_lock_held_by_current_thread;
rwlockIsOwnedByCurrentThread = __nx_sys_sync__rwlock_is_owned_by_current_thread;

/* Semaphore */
EXTERN(__nx_sys_sync__semaphore_init);
EXTERN(__nx_sys_sync__semaphore_signal);
EXTERN(__nx_sys_sync__semaphore_wait);
EXTERN(__nx_sys_sync__semaphore_try_wait);

semaphoreInit = __nx_sys_sync__semaphore_init;
semaphoreSignal = __nx_sys_sync__semaphore_signal;
semaphoreWait = __nx_sys_sync__semaphore_wait;
semaphoreTryWait = __nx_sys_sync__semaphore_try_wait;
