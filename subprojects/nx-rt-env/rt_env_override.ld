/* Static linker script for runtime environment symbols redirection */
/* Redirects libnx env functions to nx-rt-env __nx_rt_env__* implementations */

EXTERN(__nx_rt_env__envSetup);
EXTERN(__nx_rt_env__envGetLoaderInfo);
EXTERN(__nx_rt_env__envGetLoaderInfoSize);
EXTERN(__nx_rt_env__envGetMainThreadHandle);
EXTERN(__nx_rt_env__envIsNso);
EXTERN(__nx_rt_env__envHasHeapOverride);
EXTERN(__nx_rt_env__envGetHeapOverrideAddr);
EXTERN(__nx_rt_env__envGetHeapOverrideSize);
EXTERN(__nx_rt_env__envHasArgv);
EXTERN(__nx_rt_env__envGetArgv);
EXTERN(__nx_rt_env__envIsSyscallHinted);
EXTERN(__nx_rt_env__envGetOwnProcessHandle);
EXTERN(__nx_rt_env__envGetExitFuncPtr);
EXTERN(__nx_rt_env__envSetExitFuncPtr);
EXTERN(__nx_rt_env__envSetNextLoad);
EXTERN(__nx_rt_env__envHasNextLoad);
EXTERN(__nx_rt_env__envGetLastLoadResult);
EXTERN(__nx_rt_env__envHasRandomSeed);
EXTERN(__nx_rt_env__envGetRandomSeed);
EXTERN(__nx_rt_env__envGetUserIdStorage);

envSetup = __nx_rt_env__envSetup;
envGetLoaderInfo = __nx_rt_env__envGetLoaderInfo;
envGetLoaderInfoSize = __nx_rt_env__envGetLoaderInfoSize;
envGetMainThreadHandle = __nx_rt_env__envGetMainThreadHandle;
envIsNso = __nx_rt_env__envIsNso;
envHasHeapOverride = __nx_rt_env__envHasHeapOverride;
envGetHeapOverrideAddr = __nx_rt_env__envGetHeapOverrideAddr;
envGetHeapOverrideSize = __nx_rt_env__envGetHeapOverrideSize;
envHasArgv = __nx_rt_env__envHasArgv;
envGetArgv = __nx_rt_env__envGetArgv;
envIsSyscallHinted = __nx_rt_env__envIsSyscallHinted;
envGetOwnProcessHandle = __nx_rt_env__envGetOwnProcessHandle;
envGetExitFuncPtr = __nx_rt_env__envGetExitFuncPtr;
envSetExitFuncPtr = __nx_rt_env__envSetExitFuncPtr;
envSetNextLoad = __nx_rt_env__envSetNextLoad;
envHasNextLoad = __nx_rt_env__envHasNextLoad;
envGetLastLoadResult = __nx_rt_env__envGetLastLoadResult;
envHasRandomSeed = __nx_rt_env__envHasRandomSeed;
envGetRandomSeed = __nx_rt_env__envGetRandomSeed;
envGetUserIdStorage = __nx_rt_env__envGetUserIdStorage;
