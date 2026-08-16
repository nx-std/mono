/*
 * SSL ABI override: redirects libnx's `ssl` client to the Rust FFI in
 * `nx-tls` (`src/ffi/`). libnx counterparts: `src/nx/source/services/ssl.c`
 * for everything but the three `socketSslConnection*` hand-offs, which
 * upstream keeps in `src/nx/source/runtime/devices/socket.c`.
 *
 * The whole client is overridden rather than part of it: leaving one symbol
 * out would pull libnx's `ssl.o` back into the link to define it, and with it
 * a second definition of every other symbol in the file.
 *
 * Organization:
 *   - Rust FFI: subprojects/nx-tls/src/ffi/
 */

/* The service itself: bringing it up, and the commands it answers */
EXTERN(__nx_tls__sslInitialize);
EXTERN(__nx_tls__sslExit);
EXTERN(__nx_tls__sslGetServiceSession);
EXTERN(__nx_tls__sslCreateContext);
EXTERN(__nx_tls__sslGetContextCount);
EXTERN(__nx_tls__sslGetCertificates);
EXTERN(__nx_tls__sslGetCertificateBufSize);
EXTERN(__nx_tls__sslFlushSessionCache);
EXTERN(__nx_tls__sslSetDebugOption);
EXTERN(__nx_tls__sslGetDebugOption);
EXTERN(__nx_tls__sslClearTls12FallbackFlag);
EXTERN(__nx_tls__sslSetThreadCoreMask);
EXTERN(__nx_tls__sslGetThreadCoreMask);

sslInitialize             = __nx_tls__sslInitialize;
sslExit                   = __nx_tls__sslExit;
sslGetServiceSession      = __nx_tls__sslGetServiceSession;
sslCreateContext          = __nx_tls__sslCreateContext;
sslGetContextCount        = __nx_tls__sslGetContextCount;
sslGetCertificates        = __nx_tls__sslGetCertificates;
sslGetCertificateBufSize  = __nx_tls__sslGetCertificateBufSize;
sslFlushSessionCache      = __nx_tls__sslFlushSessionCache;
sslSetDebugOption         = __nx_tls__sslSetDebugOption;
sslGetDebugOption         = __nx_tls__sslGetDebugOption;
sslClearTls12FallbackFlag = __nx_tls__sslClearTls12FallbackFlag;
sslSetThreadCoreMask      = __nx_tls__sslSetThreadCoreMask;
sslGetThreadCoreMask      = __nx_tls__sslGetThreadCoreMask;

/* ISslContext */
EXTERN(__nx_tls__sslContextClose);
EXTERN(__nx_tls__sslContextSetOption);
EXTERN(__nx_tls__sslContextGetOption);
EXTERN(__nx_tls__sslContextCreateConnection);
EXTERN(__nx_tls__sslContextCreateConnectionForSystem);
EXTERN(__nx_tls__sslContextGetConnectionCount);
EXTERN(__nx_tls__sslContextImportServerPki);
EXTERN(__nx_tls__sslContextImportClientPki);
EXTERN(__nx_tls__sslContextRemovePki);
EXTERN(__nx_tls__sslContextRegisterInternalPki);
EXTERN(__nx_tls__sslContextAddPolicyOid);
EXTERN(__nx_tls__sslContextImportCrl);
EXTERN(__nx_tls__sslContextImportClientCertKeyPki);
EXTERN(__nx_tls__sslContextGeneratePrivateKeyAndCert);

sslContextClose                     = __nx_tls__sslContextClose;
sslContextSetOption                 = __nx_tls__sslContextSetOption;
sslContextGetOption                 = __nx_tls__sslContextGetOption;
sslContextCreateConnection          = __nx_tls__sslContextCreateConnection;
sslContextCreateConnectionForSystem = __nx_tls__sslContextCreateConnectionForSystem;
sslContextGetConnectionCount        = __nx_tls__sslContextGetConnectionCount;
sslContextImportServerPki           = __nx_tls__sslContextImportServerPki;
sslContextImportClientPki           = __nx_tls__sslContextImportClientPki;
sslContextRemovePki                 = __nx_tls__sslContextRemovePki;
sslContextRegisterInternalPki       = __nx_tls__sslContextRegisterInternalPki;
sslContextAddPolicyOid              = __nx_tls__sslContextAddPolicyOid;
sslContextImportCrl                 = __nx_tls__sslContextImportCrl;
sslContextImportClientCertKeyPki    = __nx_tls__sslContextImportClientCertKeyPki;
sslContextGeneratePrivateKeyAndCert = __nx_tls__sslContextGeneratePrivateKeyAndCert;

/* ISslConnection */
EXTERN(__nx_tls__sslConnectionClose);
EXTERN(__nx_tls__sslConnectionSetSocketDescriptor);
EXTERN(__nx_tls__sslConnectionGetSocketDescriptor);
EXTERN(__nx_tls__sslConnectionSetDtlsSocketDescriptor);
EXTERN(__nx_tls__sslConnectionSetHostName);
EXTERN(__nx_tls__sslConnectionSetVerifyOption);
EXTERN(__nx_tls__sslConnectionSetIoMode);
EXTERN(__nx_tls__sslConnectionGetHostName);
EXTERN(__nx_tls__sslConnectionGetVerifyOption);
EXTERN(__nx_tls__sslConnectionGetIoMode);
EXTERN(__nx_tls__sslConnectionDoHandshake);
EXTERN(__nx_tls__sslConnectionGetServerCertDetail);
EXTERN(__nx_tls__sslConnectionRead);
EXTERN(__nx_tls__sslConnectionWrite);
EXTERN(__nx_tls__sslConnectionPending);
EXTERN(__nx_tls__sslConnectionPeek);
EXTERN(__nx_tls__sslConnectionPoll);
EXTERN(__nx_tls__sslConnectionGetVerifyCertError);
EXTERN(__nx_tls__sslConnectionGetNeededServerCertBufferSize);
EXTERN(__nx_tls__sslConnectionSetSessionCacheMode);
EXTERN(__nx_tls__sslConnectionGetSessionCacheMode);
EXTERN(__nx_tls__sslConnectionFlushSessionCache);
EXTERN(__nx_tls__sslConnectionSetRenegotiationMode);
EXTERN(__nx_tls__sslConnectionGetRenegotiationMode);
EXTERN(__nx_tls__sslConnectionSetOption);
EXTERN(__nx_tls__sslConnectionGetOption);
EXTERN(__nx_tls__sslConnectionGetVerifyCertErrors);
EXTERN(__nx_tls__sslConnectionGetCipherInfo);
EXTERN(__nx_tls__sslConnectionSetNextAlpnProto);
EXTERN(__nx_tls__sslConnectionGetNextAlpnProto);
EXTERN(__nx_tls__sslConnectionGetDtlsHandshakeTimeout);
EXTERN(__nx_tls__sslConnectionSetPrivateOption);
EXTERN(__nx_tls__sslConnectionSetSrtpCiphers);
EXTERN(__nx_tls__sslConnectionGetSrtpCipher);
EXTERN(__nx_tls__sslConnectionExportKeyingMaterial);
EXTERN(__nx_tls__sslConnectionSetIoTimeout);
EXTERN(__nx_tls__sslConnectionGetIoTimeout);

sslConnectionClose                         = __nx_tls__sslConnectionClose;
sslConnectionSetSocketDescriptor           = __nx_tls__sslConnectionSetSocketDescriptor;
sslConnectionGetSocketDescriptor           = __nx_tls__sslConnectionGetSocketDescriptor;
sslConnectionSetDtlsSocketDescriptor       = __nx_tls__sslConnectionSetDtlsSocketDescriptor;
sslConnectionSetHostName                   = __nx_tls__sslConnectionSetHostName;
sslConnectionSetVerifyOption               = __nx_tls__sslConnectionSetVerifyOption;
sslConnectionSetIoMode                     = __nx_tls__sslConnectionSetIoMode;
sslConnectionGetHostName                   = __nx_tls__sslConnectionGetHostName;
sslConnectionGetVerifyOption               = __nx_tls__sslConnectionGetVerifyOption;
sslConnectionGetIoMode                     = __nx_tls__sslConnectionGetIoMode;
sslConnectionDoHandshake                   = __nx_tls__sslConnectionDoHandshake;
sslConnectionGetServerCertDetail           = __nx_tls__sslConnectionGetServerCertDetail;
sslConnectionRead                          = __nx_tls__sslConnectionRead;
sslConnectionWrite                         = __nx_tls__sslConnectionWrite;
sslConnectionPending                       = __nx_tls__sslConnectionPending;
sslConnectionPeek                          = __nx_tls__sslConnectionPeek;
sslConnectionPoll                          = __nx_tls__sslConnectionPoll;
sslConnectionGetVerifyCertError            = __nx_tls__sslConnectionGetVerifyCertError;
sslConnectionGetNeededServerCertBufferSize = __nx_tls__sslConnectionGetNeededServerCertBufferSize;
sslConnectionSetSessionCacheMode           = __nx_tls__sslConnectionSetSessionCacheMode;
sslConnectionGetSessionCacheMode           = __nx_tls__sslConnectionGetSessionCacheMode;
sslConnectionFlushSessionCache             = __nx_tls__sslConnectionFlushSessionCache;
sslConnectionSetRenegotiationMode          = __nx_tls__sslConnectionSetRenegotiationMode;
sslConnectionGetRenegotiationMode          = __nx_tls__sslConnectionGetRenegotiationMode;
sslConnectionSetOption                     = __nx_tls__sslConnectionSetOption;
sslConnectionGetOption                     = __nx_tls__sslConnectionGetOption;
sslConnectionGetVerifyCertErrors           = __nx_tls__sslConnectionGetVerifyCertErrors;
sslConnectionGetCipherInfo                 = __nx_tls__sslConnectionGetCipherInfo;
sslConnectionSetNextAlpnProto              = __nx_tls__sslConnectionSetNextAlpnProto;
sslConnectionGetNextAlpnProto              = __nx_tls__sslConnectionGetNextAlpnProto;
sslConnectionGetDtlsHandshakeTimeout       = __nx_tls__sslConnectionGetDtlsHandshakeTimeout;
sslConnectionSetPrivateOption              = __nx_tls__sslConnectionSetPrivateOption;
sslConnectionSetSrtpCiphers                = __nx_tls__sslConnectionSetSrtpCiphers;
sslConnectionGetSrtpCipher                 = __nx_tls__sslConnectionGetSrtpCipher;
sslConnectionExportKeyingMaterial          = __nx_tls__sslConnectionExportKeyingMaterial;
sslConnectionSetIoTimeout                  = __nx_tls__sslConnectionSetIoTimeout;
sslConnectionGetIoTimeout                  = __nx_tls__sslConnectionGetIoTimeout;

/* Giving a socket to a connection, from the socket driver's side */
EXTERN(__nx_tls__socketSslConnectionSetSocketDescriptor);
EXTERN(__nx_tls__socketSslConnectionGetSocketDescriptor);
EXTERN(__nx_tls__socketSslConnectionSetDtlsSocketDescriptor);

socketSslConnectionSetSocketDescriptor     = __nx_tls__socketSslConnectionSetSocketDescriptor;
socketSslConnectionGetSocketDescriptor     = __nx_tls__socketSslConnectionGetSocketDescriptor;
socketSslConnectionSetDtlsSocketDescriptor = __nx_tls__socketSslConnectionSetDtlsSocketDescriptor;
