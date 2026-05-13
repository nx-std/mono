//! SSL service protocol constants.

use nx_sf::ServiceName;

/// Service name for the default SSL service (`ssl`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("ssl");

/// Service name for the system SSL service (`ssl:s`, 15.0.0+).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("ssl:s");

// ISslService commands

/// CreateContext (sends PID).
pub const CREATE_CONTEXT: u32 = 0;

/// CreateContext for system service variant (sends PID).
pub const CREATE_CONTEXT_SYSTEM: u32 = 100;

/// GetContextCount.
pub const GET_CONTEXT_COUNT: u32 = 1;

/// GetCertificates (in buffer + out buffer).
pub const GET_CERTIFICATES: u32 = 2;

/// GetCertificateBufSize (in buffer).
pub const GET_CERTIFICATE_BUF_SIZE: u32 = 3;

/// SetInterfaceVersion (3.0.0+, internal).
pub const SET_INTERFACE_VERSION: u32 = 5;

/// FlushSessionCache (5.0.0+).
pub const FLUSH_SESSION_CACHE: u32 = 6;

/// SetDebugOption (6.0.0+).
pub const SET_DEBUG_OPTION: u32 = 7;

/// GetDebugOption (6.0.0+).
pub const GET_DEBUG_OPTION: u32 = 8;

/// ClearTls12FallbackFlag (14.0.0+).
pub const CLEAR_TLS12_FALLBACK_FLAG: u32 = 9;

/// SetThreadCoreMask (15.0.0+, system only).
pub const SET_THREAD_CORE_MASK: u32 = 101;

/// GetThreadCoreMask (15.0.0+, system only).
pub const GET_THREAD_CORE_MASK: u32 = 102;

// ISslContext commands

/// SetOption (context).
pub const CTX_SET_OPTION: u32 = 0;

/// GetOption (context).
pub const CTX_GET_OPTION: u32 = 1;

/// CreateConnection (context).
pub const CTX_CREATE_CONNECTION: u32 = 2;

/// GetConnectionCount (context).
pub const CTX_GET_CONNECTION_COUNT: u32 = 3;

/// ImportServerPki (context).
pub const CTX_IMPORT_SERVER_PKI: u32 = 4;

/// ImportClientPki (context).
pub const CTX_IMPORT_CLIENT_PKI: u32 = 5;

/// RemoveServerPki (context).
pub const CTX_REMOVE_SERVER_PKI: u32 = 6;

/// RemoveClientPki (context).
pub const CTX_REMOVE_CLIENT_PKI: u32 = 7;

/// RegisterInternalPki (context).
pub const CTX_REGISTER_INTERNAL_PKI: u32 = 8;

/// AddPolicyOid (context).
pub const CTX_ADD_POLICY_OID: u32 = 9;

/// ImportCrl (context, 3.0.0+).
pub const CTX_IMPORT_CRL: u32 = 10;

/// RemoveCrl (context, 3.0.0+).
pub const CTX_REMOVE_CRL: u32 = 11;

/// ImportClientCertKeyPki (context, 16.0.0+).
pub const CTX_IMPORT_CLIENT_CERT_KEY_PKI: u32 = 12;

/// GeneratePrivateKeyAndCert (context, 16.0.0+).
pub const CTX_GENERATE_PRIVATE_KEY_AND_CERT: u32 = 13;

/// CreateConnectionForSystem (context, 15.0.0+ system only).
pub const CTX_CREATE_CONNECTION_FOR_SYSTEM: u32 = 100;

// ISslConnection commands

/// SetSocketDescriptor (connection).
pub const CONN_SET_SOCKET_DESCRIPTOR: u32 = 0;

/// SetHostName (connection).
pub const CONN_SET_HOST_NAME: u32 = 1;

/// SetVerifyOption (connection).
pub const CONN_SET_VERIFY_OPTION: u32 = 2;

/// SetIoMode (connection).
pub const CONN_SET_IO_MODE: u32 = 3;

/// GetSocketDescriptor (connection).
pub const CONN_GET_SOCKET_DESCRIPTOR: u32 = 4;

/// GetHostName (connection).
pub const CONN_GET_HOST_NAME: u32 = 5;

/// GetVerifyOption (connection).
pub const CONN_GET_VERIFY_OPTION: u32 = 6;

/// GetIoMode (connection).
pub const CONN_GET_IO_MODE: u32 = 7;

/// DoHandshake (connection).
pub const CONN_DO_HANDSHAKE: u32 = 8;

/// DoHandshakeGetServerCert (connection).
pub const CONN_DO_HANDSHAKE_GET_SERVER_CERT: u32 = 9;

/// Read (connection).
pub const CONN_READ: u32 = 10;

/// Write (connection).
pub const CONN_WRITE: u32 = 11;

/// Pending (connection).
pub const CONN_PENDING: u32 = 12;

/// Peek (connection).
pub const CONN_PEEK: u32 = 13;

/// Poll (connection).
pub const CONN_POLL: u32 = 14;

/// GetVerifyCertError (connection).
pub const CONN_GET_VERIFY_CERT_ERROR: u32 = 15;

/// GetNeededServerCertBufferSize (connection).
pub const CONN_GET_NEEDED_SERVER_CERT_BUFFER_SIZE: u32 = 16;

/// SetSessionCacheMode (connection).
pub const CONN_SET_SESSION_CACHE_MODE: u32 = 17;

/// GetSessionCacheMode (connection).
pub const CONN_GET_SESSION_CACHE_MODE: u32 = 18;

/// FlushSessionCache (connection).
pub const CONN_FLUSH_SESSION_CACHE: u32 = 19;

/// SetRenegotiationMode (connection).
pub const CONN_SET_RENEGOTIATION_MODE: u32 = 20;

/// GetRenegotiationMode (connection).
pub const CONN_GET_RENEGOTIATION_MODE: u32 = 21;

/// SetOption (connection).
pub const CONN_SET_OPTION: u32 = 22;

/// GetOption (connection).
pub const CONN_GET_OPTION: u32 = 23;

/// GetVerifyCertErrors (connection).
pub const CONN_GET_VERIFY_CERT_ERRORS: u32 = 24;

/// GetCipherInfo (connection, 4.0.0+).
pub const CONN_GET_CIPHER_INFO: u32 = 25;

/// SetNextAlpnProto (connection, 9.0.0+).
pub const CONN_SET_NEXT_ALPN_PROTO: u32 = 26;

/// GetNextAlpnProto (connection, 9.0.0+).
pub const CONN_GET_NEXT_ALPN_PROTO: u32 = 27;

/// SetDtlsSocketDescriptor (connection, 16.0.0+).
pub const CONN_SET_DTLS_SOCKET_DESCRIPTOR: u32 = 28;

/// GetDtlsHandshakeTimeout (connection, 16.0.0+).
pub const CONN_GET_DTLS_HANDSHAKE_TIMEOUT: u32 = 29;

/// SetPrivateOption (connection, 16.0.0+).
pub const CONN_SET_PRIVATE_OPTION: u32 = 30;

/// SetSrtpCiphers (connection, 16.0.0+).
pub const CONN_SET_SRTP_CIPHERS: u32 = 31;

/// GetSrtpCipher (connection, 16.0.0+).
pub const CONN_GET_SRTP_CIPHER: u32 = 32;

/// ExportKeyingMaterial (connection, 16.0.0+).
pub const CONN_EXPORT_KEYING_MATERIAL: u32 = 33;

/// SetIoTimeout (connection, 16.0.0+).
pub const CONN_SET_IO_TIMEOUT: u32 = 34;

/// GetIoTimeout (connection, 16.0.0+).
pub const CONN_GET_IO_TIMEOUT: u32 = 35;
