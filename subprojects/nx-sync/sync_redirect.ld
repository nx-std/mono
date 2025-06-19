/* Static linker script for sync function symbols redirection */
/* Redirects libnx functions to nx-sync __nx_sync_* implementations */

/* Barrier */
EXTERN(__nx_sync_barrier_init);
EXTERN(__nx_sync_barrier_wait);

barrierInit = __nx_sync_barrier_init;
barrierWait = __nx_sync_barrier_wait;

/* CondVar */
EXTERN(__nx_sync_condvar_init);
EXTERN(__nx_sync_condvar_wait_timeout);
EXTERN(__nx_sync_condvar_wait);
EXTERN(__nx_sync_condvar_wake);
EXTERN(__nx_sync_condvar_wake_one);
EXTERN(__nx_sync_condvar_wake_all);

condvarInit = __nx_sync_condvar_init;
condvarWaitTimeout = __nx_sync_condvar_wait_timeout;
condvarWait = __nx_sync_condvar_wait;
condvarWake = __nx_sync_condvar_wake;
condvarWakeOne = __nx_sync_condvar_wake_one;
condvarWakeAll = __nx_sync_condvar_wake_all;

/* Mutex */
EXTERN(__nx_sync_mutex_init);
EXTERN(__nx_sync_mutex_lock);
EXTERN(__nx_sync_mutex_try_lock);
EXTERN(__nx_sync_mutex_unlock);
EXTERN(__nx_sync_mutex_is_locked_by_current_thread);

mutexInit = __nx_sync_mutex_init;
mutexLock = __nx_sync_mutex_lock;
mutexTryLock = __nx_sync_mutex_try_lock;
mutexUnlock = __nx_sync_mutex_unlock;
mutexIsLockedByCurrentThread = __nx_sync_mutex_is_locked_by_current_thread;

/* RMutex */
EXTERN(__nx_sync_remutex_init);
EXTERN(__nx_sync_remutex_lock);
EXTERN(__nx_sync_remutex_try_lock);
EXTERN(__nx_sync_remutex_unlock);

rmutexInit = __nx_sync_remutex_init;
rmutexLock = __nx_sync_remutex_lock;
rmutexTryLock = __nx_sync_remutex_try_lock;
rmutexUnlock = __nx_sync_remutex_unlock;

/* RwLock */
EXTERN(__nx_sync_rwlock_init);
EXTERN(__nx_sync_rwlock_read_lock);
EXTERN(__nx_sync_rwlock_try_read_lock);
EXTERN(__nx_sync_rwlock_read_unlock);
EXTERN(__nx_sync_rwlock_write_lock);
EXTERN(__nx_sync_rwlock_try_write_lock);
EXTERN(__nx_sync_rwlock_write_unlock);
EXTERN(__nx_sync_rwlock_is_write_lock_held_by_current_thread);
EXTERN(__nx_sync_rwlock_is_owned_by_current_thread);

rwlockInit = __nx_sync_rwlock_init;
rwlockReadLock = __nx_sync_rwlock_read_lock;
rwlockTryReadLock = __nx_sync_rwlock_try_read_lock;
rwlockReadUnlock = __nx_sync_rwlock_read_unlock;
rwlockWriteLock = __nx_sync_rwlock_write_lock;
rwlockTryWriteLock = __nx_sync_rwlock_try_write_lock;
rwlockWriteUnlock = __nx_sync_rwlock_write_unlock;
rwlockIsWriteLockHeldByCurrentThread = __nx_sync_rwlock_is_write_lock_held_by_current_thread;
rwlockIsOwnedByCurrentThread = __nx_sync_rwlock_is_owned_by_current_thread;

/* Semaphore */
EXTERN(__nx_sync_semaphore_init);
EXTERN(__nx_sync_semaphore_signal);
EXTERN(__nx_sync_semaphore_wait);
EXTERN(__nx_sync_semaphore_try_wait);

semaphoreInit = __nx_sync_semaphore_init;
semaphoreSignal = __nx_sync_semaphore_signal;
semaphoreWait = __nx_sync_semaphore_wait;
semaphoreTryWait = __nx_sync_semaphore_try_wait;
