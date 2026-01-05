/* nx-sf linker override script
 *
 * NOTE: libnx service functions (serviceCreate, serviceClose, etc.) are
 * NX_INLINE and have no linkable symbols. These overrides are provided for:
 * - Explicit use of the Rust implementation from C code
 * - Future compatibility if libnx makes them non-inline
 *
 * The actual IPC override happens at the SVC layer (nx-svc).
 */

/* Ensure Rust symbols are pulled in */
EXTERN(__nx_sf__service_create);
EXTERN(__nx_sf__service_create_non_domain_subservice);
EXTERN(__nx_sf__service_create_domain_subservice);
EXTERN(__nx_sf__service_close);
EXTERN(__nx_sf__service_clone);
EXTERN(__nx_sf__service_clone_ex);
EXTERN(__nx_sf__service_convert_to_domain);
EXTERN(__nx_sf__service_is_active);
EXTERN(__nx_sf__service_is_override);
EXTERN(__nx_sf__service_is_domain);
EXTERN(__nx_sf__service_is_domain_subservice);
EXTERN(__nx_sf__service_get_object_id);

/* Redirect libnx symbols to Rust implementations
 * (These only work if libnx is compiled without inlining) */
serviceCreate = __nx_sf__service_create;
serviceCreateNonDomainSubservice = __nx_sf__service_create_non_domain_subservice;
serviceCreateDomainSubservice = __nx_sf__service_create_domain_subservice;
serviceClose = __nx_sf__service_close;
serviceClone = __nx_sf__service_clone;
serviceCloneEx = __nx_sf__service_clone_ex;
serviceConvertToDomain = __nx_sf__service_convert_to_domain;
serviceIsActive = __nx_sf__service_is_active;
serviceIsOverride = __nx_sf__service_is_override;
serviceIsDomain = __nx_sf__service_is_domain;
serviceIsDomainSubservice = __nx_sf__service_is_domain_subservice;
serviceGetObjectId = __nx_sf__service_get_object_id;
