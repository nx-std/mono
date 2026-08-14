//! Nintendo Shell (`ns`) service family implementation.
//!
//! Provides access to the NS service family: application management, content
//! management, factory reset, download tasks, system updates, and developer
//! tools.
//!
//! ## Service Variants
//!
//! Four independent service endpoints:
//!
//! - **`ns:am` / `ns:am2`** — Main NS service. On pre-3.0.0 firmware,
//!   `ns:am` provides `IApplicationManagerInterface` directly. On 3.0.0+,
//!   `ns:am2` (or other `ns:*` fallbacks) provides a getter interface that
//!   produces sub-interface sessions on demand.
//! - **`ns:vm`** — Version management. Connected via [`connect_nsvm_cmif`].
//! - **`ns:dev`** — Developer service. Connected via [`connect_nsdev_cmif`].
//! - **`ns:su`** — System update. Connected via [`connect_nssu_cmif`].
//!
//! ## Architecture (main ns service)
//!
//! The main NS service has two connection modes per IC-4:
//!
//! - [`connect_cmif_legacy`] → connects to `ns:am`, returns an
//!   [`NsAppManagerService`] that wraps `IApplicationManagerInterface`
//!   directly (pre-3.0.0).
//! - [`connect_cmif`] → connects to `ns:am2` (with `ns:ec`/`ns:web`/
//!   `ns:rid`/`ns:rt`/`ns:ro` fallbacks), returns an [`NsGetterService`]
//!   whose methods produce sub-interface sessions: application manager,
//!   content management, download task, factory reset, ecommerce, etc.
//!
//! Sub-interfaces obtained from the getter are wrapped in their own
//! service types ([`NsAppManagerService`], [`NsFactoryResetService`],
//! [`NsECommerceService`], etc.) that implement the appropriate IPC
//! commands.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    DispatchError,
    OwnedSessionHandle,
    Session,
};
use nx_svc::ipc::Handle;

mod cmif;
mod dispatch;
mod proto;
pub mod types;

pub use self::{
    cmif::{
        app_manager::{
            AcquireEventError,
            AsyncCommandError,
            AsyncOut,
            GetSubObjectError,
        },
        getter::GetInterfaceError,
        nsdev::AcquireEventError as NsdevAcquireEventError,
        nssu::{
            AcquireEventError as NssuAcquireEventError,
            OpenSystemUpdateControlError,
        },
        sub_objects::AcquireEventError as SubObjectAcquireEventError,
    },
    proto::{
        NS_AM_SERVICE_NAME,
        NS_AM2_SERVICE_NAME,
        NS_EC_SERVICE_NAME,
        NS_RID_SERVICE_NAME,
        NS_RO_SERVICE_NAME,
        NS_RT_SERVICE_NAME,
        NS_WEB_SERVICE_NAME,
        NSDEV_SERVICE_NAME,
        NSSU_SERVICE_NAME,
        NSVM_SERVICE_NAME,
    },
    types::{
        AccountUid,
        ApplicationContentMetaStatus,
        ApplicationControlData,
        ApplicationControlSource,
        ApplicationDeliveryInfo,
        ApplicationOccupiedSize,
        ApplicationRecord,
        ApplicationRightsOnClient,
        ApplicationView,
        ApplicationViewDeprecated,
        ApplicationViewWithPromotionInfo,
        BackgroundNetworkUpdateState,
        DownloadTaskStatus,
        EulaDataPath,
        LatestSystemUpdate,
        LaunchProperties,
        NcmContentMetaKey,
        ProgressForDeleteUserSaveDataAll,
        PromotionInfo,
        ReceiveApplicationProgress,
        SendApplicationProgress,
        ShellEvent,
        ShellEventInfo,
        SystemDeliveryInfo,
        SystemUpdateProgress,
    },
};

// ===========================================================================
// NsGetterService — getter interface (ns:am2, 3.0.0+)
// ===========================================================================

/// NS getter service wrapper (`ns:am2` / `ns:ec` / `ns:web` / etc.).
///
/// On 3.0.0+, this is the primary entry point. Use its methods to obtain
/// sub-interface sessions for application management, content management,
/// ecommerce, factory reset, and other functionality.
#[repr(transparent)]
pub struct NsGetterService(Session);

impl NsGetterService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    pub fn get_application_manager_interface(
        &self,
    ) -> Result<NsAppManagerService, GetInterfaceError> {
        let raw = cmif::getter::get_application_manager_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsAppManagerService(Session::new(handle, 0)))
    }

    pub fn get_content_management_interface(
        &self,
    ) -> Result<NsContentManagementService, GetInterfaceError> {
        let raw = cmif::getter::get_content_management_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsContentManagementService(Session::new(handle, 0)))
    }

    pub fn get_download_task_interface(&self) -> Result<NsDownloadTaskService, GetInterfaceError> {
        let raw = cmif::getter::get_download_task_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsDownloadTaskService(Session::new(handle, 0)))
    }

    pub fn get_factory_reset_interface(&self) -> Result<NsFactoryResetService, GetInterfaceError> {
        let raw = cmif::getter::get_factory_reset_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsFactoryResetService(Session::new(handle, 0)))
    }

    pub fn get_ecommerce_interface(&self) -> Result<NsECommerceService, GetInterfaceError> {
        let raw = cmif::getter::get_ecommerce_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsECommerceService(Session::new(handle, 0)))
    }

    pub fn get_readonly_application_control_data_interface(
        &self,
    ) -> Result<NsReadOnlyControlDataService, GetInterfaceError> {
        let raw = cmif::getter::get_readonly_application_control_data_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsReadOnlyControlDataService(Session::new(handle, 0)))
    }

    pub fn get_readonly_application_record_interface(
        &self,
    ) -> Result<NsReadOnlyRecordService, GetInterfaceError> {
        let raw = cmif::getter::get_readonly_application_record_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsReadOnlyRecordService(Session::new(handle, 0)))
    }

    pub fn get_dynamic_rights_interface(
        &self,
    ) -> Result<NsDynamicRightsService, GetInterfaceError> {
        let raw = cmif::getter::get_dynamic_rights_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsDynamicRightsService(Session::new(handle, 0)))
    }

    pub fn get_application_version_interface(
        &self,
    ) -> Result<NsApplicationVersionService, GetInterfaceError> {
        let raw = cmif::getter::get_application_version_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsApplicationVersionService(Session::new(handle, 0)))
    }

    pub fn get_account_proxy_interface(&self) -> Result<NsAccountProxyService, GetInterfaceError> {
        let raw = cmif::getter::get_account_proxy_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsAccountProxyService(Session::new(handle, 0)))
    }

    pub fn get_document_interface(&self) -> Result<NsDocumentService, GetInterfaceError> {
        let raw = cmif::getter::get_document_interface(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsDocumentService(Session::new(handle, 0)))
    }
}

/// Connects to the NS getter interface (`ns:am2`, 3.0.0+).
///
/// On 3.0.0+, the official software tries `ns:am2` first, falling back
/// through `ns:ro`, `ns:rt`, `ns:rid`, `ns:web`, `ns:ec`. This function
/// connects to `ns:am2` only. Use [`connect_cmif_fallback`] to try
/// alternative service names.
pub fn connect_cmif(sm: &SmService) -> Result<NsGetterService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(NS_AM2_SERVICE_NAME)
        .map_err(ConnectCmifError)?;
    Ok(NsGetterService(Session::new(handle, 0)))
}

/// Connects to the NS getter interface using a specified service name.
///
/// Useful for fallback: try `ns:am2`, then `ns:ro` (11.0.0+), `ns:rt`,
/// `ns:rid`, `ns:web`, `ns:ec`.
pub fn connect_cmif_fallback(
    sm: &SmService,
    service_name: nx_service_sm::ServiceName,
) -> Result<NsGetterService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectCmifError)?;
    Ok(NsGetterService(Session::new(handle, 0)))
}

/// Connects to the NS legacy service (`ns:am`, pre-3.0.0).
///
/// Returns an [`NsAppManagerService`] wrapping `IApplicationManagerInterface`
/// directly, without the getter indirection.
pub fn connect_cmif_legacy(sm: &SmService) -> Result<NsAppManagerService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(NS_AM_SERVICE_NAME)
        .map_err(ConnectCmifError)?;
    Ok(NsAppManagerService(Session::new(handle, 0)))
}

#[derive(Debug, thiserror::Error)]
#[error("failed to get ns service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

// ===========================================================================
// NsAppManagerService — IApplicationManagerInterface
// ===========================================================================

/// Application manager service wrapper.
///
/// Obtained via [`NsGetterService::get_application_manager_interface`] (3.0.0+)
/// or [`connect_cmif_legacy`] (pre-3.0.0).
///
/// On pre-3.0.0, this also serves as the dispatch target for download task,
/// content management, and factory reset commands (same session, same cmd IDs).
#[repr(transparent)]
pub struct NsAppManagerService(Session);

impl NsAppManagerService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// No I/O

impl NsAppManagerService {
    #[inline]
    pub fn delete_redundant_application_entity(&self) -> Result<(), DispatchError> {
        cmif::app_manager::delete_redundant_application_entity(&self.0)
    }

    #[inline]
    pub fn cleanup_sd_card(&self) -> Result<(), DispatchError> {
        cmif::app_manager::cleanup_sd_card(&self.0)
    }

    #[inline]
    pub fn check_sd_card_mount_status(&self) -> Result<(), DispatchError> {
        cmif::app_manager::check_sd_card_mount_status(&self.0)
    }

    #[inline]
    pub fn get_last_sd_card_mount_unexpected_result(&self) -> Result<(), DispatchError> {
        cmif::app_manager::get_last_sd_card_mount_unexpected_result(&self.0)
    }

    #[inline]
    pub fn resume_all(&self) -> Result<(), DispatchError> {
        cmif::app_manager::resume_all(&self.0)
    }

    #[inline]
    pub fn ensure_game_card_access(&self) -> Result<(), DispatchError> {
        cmif::app_manager::ensure_game_card_access(&self.0)
    }

    #[inline]
    pub fn get_last_game_card_mount_failure_result(&self) -> Result<(), DispatchError> {
        cmif::app_manager::get_last_game_card_mount_failure_result(&self.0)
    }

    #[inline]
    pub fn format_sd_card(&self) -> Result<(), DispatchError> {
        cmif::app_manager::format_sd_card(&self.0)
    }

    #[inline]
    pub fn clear_task_status_list(&self) -> Result<(), DispatchError> {
        cmif::app_manager::clear_task_status_list(&self.0)
    }

    #[inline]
    pub fn request_download_task_list(&self) -> Result<(), DispatchError> {
        cmif::app_manager::request_download_task_list(&self.0)
    }

    #[inline]
    pub fn try_commit_current_application_download_task(&self) -> Result<(), DispatchError> {
        cmif::app_manager::try_commit_current_application_download_task(&self.0)
    }

    #[inline]
    pub fn enable_auto_commit(&self) -> Result<(), DispatchError> {
        cmif::app_manager::enable_auto_commit(&self.0)
    }

    #[inline]
    pub fn disable_auto_commit(&self) -> Result<(), DispatchError> {
        cmif::app_manager::disable_auto_commit(&self.0)
    }

    #[inline]
    pub fn trigger_dynamic_commit_event(&self) -> Result<(), DispatchError> {
        cmif::app_manager::trigger_dynamic_commit_event(&self.0)
    }
}

// u64 input

impl NsAppManagerService {
    #[inline]
    pub fn delete_application_entity(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::delete_application_entity(&self.0, application_id)
    }

    #[inline]
    pub fn delete_application_completely(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::delete_application_completely(&self.0, application_id)
    }

    #[inline]
    pub fn cancel_application_download(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::cancel_application_download(&self.0, application_id)
    }

    #[inline]
    pub fn resume_application_download(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::resume_application_download(&self.0, application_id)
    }

    #[inline]
    pub fn check_application_launch_version(
        &self,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::check_application_launch_version(&self.0, application_id)
    }

    #[inline]
    pub fn disable_application_auto_delete(
        &self,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::disable_application_auto_delete(&self.0, application_id)
    }

    #[inline]
    pub fn enable_application_auto_delete(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::enable_application_auto_delete(&self.0, application_id)
    }

    #[inline]
    pub fn clear_application_terminate_result(
        &self,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::clear_application_terminate_result(&self.0, application_id)
    }

    #[inline]
    pub fn cancel_application_apply_delta(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::cancel_application_apply_delta(&self.0, application_id)
    }

    #[inline]
    pub fn resume_application_apply_delta(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::resume_application_apply_delta(&self.0, application_id)
    }

    #[inline]
    pub fn touch_application(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::touch_application(&self.0, application_id)
    }

    #[inline]
    pub fn withdraw_application_update_request(
        &self,
        application_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::withdraw_application_update_request(&self.0, application_id)
    }

    #[inline]
    pub fn commit_receive_application(&self, application_id: u64) -> Result<(), DispatchError> {
        cmif::app_manager::commit_receive_application(&self.0, application_id)
    }
}

// Compound I/O

impl NsAppManagerService {
    pub fn calculate_application_download_required_size(
        &self,
        application_id: u64,
    ) -> Result<(u8, i64), DispatchError> {
        let out = cmif::app_manager::calculate_application_download_required_size(
            &self.0,
            application_id,
        )?;
        Ok((out.storage_id, out.size))
    }

    pub fn calculate_application_apply_delta_required_size(
        &self,
        application_id: u64,
    ) -> Result<(u8, i64), DispatchError> {
        let out = cmif::app_manager::calculate_application_apply_delta_required_size(
            &self.0,
            application_id,
        )?;
        Ok((out.storage_id, out.size))
    }

    #[inline]
    pub fn is_application_entity_movable(
        &self,
        application_id: u64,
        storage_id: u8,
    ) -> Result<bool, DispatchError> {
        cmif::app_manager::is_application_entity_movable(
            &self.0,
            types::IsEntityMovableIn {
                storage_id,
                pad: [0; 7],
                application_id,
            },
        )
    }

    #[inline]
    pub fn move_application_entity(
        &self,
        application_id: u64,
        storage_id: u8,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::move_application_entity(
            &self.0,
            types::IsEntityMovableIn {
                storage_id,
                pad: [0; 7],
                application_id,
            },
        )
    }

    #[inline]
    pub fn set_application_terminate_result(
        &self,
        application_id: u64,
        result_code: u32,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::set_application_terminate_result(
            &self.0,
            types::SetTerminateResultIn {
                result: result_code,
                pad: 0,
                application_id,
            },
        )
    }

    #[inline]
    pub fn delete_user_system_save_data(
        &self,
        uid: AccountUid,
        system_save_data_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::delete_user_system_save_data(
            &self.0,
            types::DeleteUserSystemSaveDataIn {
                uid,
                system_save_data_id,
            },
        )
    }

    #[inline]
    pub fn delete_save_data(
        &self,
        save_data_space_id: u8,
        save_data_id: u64,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::delete_save_data(
            &self.0,
            types::DeleteSaveDataIn {
                save_data_space_id,
                pad: [0; 7],
                save_data_id,
            },
        )
    }

    #[inline]
    pub fn unregister_network_service_account(&self, uid: AccountUid) -> Result<(), DispatchError> {
        cmif::app_manager::unregister_network_service_account(&self.0, uid)
    }

    #[inline]
    pub fn unregister_network_service_account_with_user_save_data_deletion(
        &self,
        uid: AccountUid,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::unregister_network_service_account_with_user_save_data_deletion(
            &self.0, uid,
        )
    }

    #[inline]
    pub fn cleanup_unavailable_addon_contents(
        &self,
        application_id: u64,
        uid: AccountUid,
    ) -> Result<(), DispatchError> {
        cmif::app_manager::cleanup_unavailable_addon_contents(
            &self.0,
            types::CleanupUnavailableAddOnContentsIn {
                application_id,
                uid,
            },
        )
    }

    #[inline]
    pub fn get_application_terminate_result(
        &self,
        application_id: u64,
    ) -> Result<u32, DispatchError> {
        cmif::app_manager::get_application_terminate_result(&self.0, application_id)
    }

    pub fn get_storage_size(&self, storage_id: u8) -> Result<(i64, i64), DispatchError> {
        let out = cmif::app_manager::get_storage_size(&self.0, storage_id)?;
        Ok((out.total_space_size, out.free_space_size))
    }

    pub fn is_application_update_requested(
        &self,
        application_id: u64,
    ) -> Result<(bool, u32), DispatchError> {
        let out = cmif::app_manager::is_application_update_requested(&self.0, application_id)?;
        Ok((out.flag != 0, out.out))
    }

    #[inline]
    pub fn count_application_content_meta(
        &self,
        application_id: u64,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::count_application_content_meta(&self.0, application_id)
    }

    #[inline]
    pub fn is_any_application_entity_installed(
        &self,
        application_id: u64,
    ) -> Result<bool, DispatchError> {
        cmif::app_manager::is_any_application_entity_installed(&self.0, application_id)
    }

    #[inline]
    pub fn is_game_card_inserted(&self, application_id: u64) -> Result<bool, DispatchError> {
        cmif::app_manager::is_game_card_inserted(&self.0, application_id)
    }

    #[inline]
    pub fn needs_system_update_to_format_sd_card(&self) -> Result<bool, DispatchError> {
        cmif::app_manager::needs_system_update_to_format_sd_card(&self.0)
    }

    #[inline]
    pub fn is_any_application_running(&self) -> Result<bool, DispatchError> {
        cmif::app_manager::is_any_application_running(&self.0)
    }

    #[inline]
    pub fn get_receive_application_progress(
        &self,
        application_id: u64,
    ) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::app_manager::get_receive_application_progress(&self.0, application_id)
    }

    #[inline]
    pub fn get_send_application_progress(
        &self,
        application_id: u64,
    ) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::app_manager::get_send_application_progress(&self.0, application_id)
    }

    pub fn calculate_application_occupied_size(
        &self,
        application_id: u64,
        out: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::calculate_application_occupied_size(&self.0, application_id, out)
    }

    #[inline]
    pub fn get_total_space_size(&self, storage_id: u64) -> Result<u64, DispatchError> {
        cmif::app_manager::get_total_space_size(&self.0, storage_id)
    }

    #[inline]
    pub fn get_free_space_size(&self, storage_id: u64) -> Result<u64, DispatchError> {
        cmif::app_manager::get_free_space_size(&self.0, storage_id)
    }
}

// Event commands

impl NsAppManagerService {
    #[inline]
    pub fn get_application_record_update_system_event(&self) -> Result<u32, AcquireEventError> {
        cmif::app_manager::get_application_record_update_system_event(&self.0)
    }

    #[inline]
    pub fn get_sd_card_mount_status_changed_event(&self) -> Result<u32, AcquireEventError> {
        cmif::app_manager::get_sd_card_mount_status_changed_event(&self.0)
    }

    #[inline]
    pub fn get_game_card_update_detection_event(&self) -> Result<u32, AcquireEventError> {
        cmif::app_manager::get_game_card_update_detection_event(&self.0)
    }

    #[inline]
    pub fn get_game_card_mount_failure_event(&self) -> Result<u32, AcquireEventError> {
        cmif::app_manager::get_game_card_mount_failure_event(&self.0)
    }
}

// Buffer commands

impl NsAppManagerService {
    #[inline]
    pub fn list_application_record(
        &self,
        records: &mut [u8],
        entry_offset: i32,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_application_record(&self.0, records, entry_offset)
    }

    #[inline]
    pub fn get_application_view_deprecated(
        &self,
        views: &mut [u8],
        application_ids: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::get_application_view_deprecated(&self.0, views, application_ids)
    }

    #[inline]
    pub fn get_application_view(
        &self,
        views: &mut [u8],
        application_ids: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::get_application_view(&self.0, views, application_ids)
    }

    #[inline]
    pub fn get_application_view_with_promotion_info(
        &self,
        views: &mut [u8],
        application_ids: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::get_application_view_with_promotion_info(&self.0, views, application_ids)
    }

    #[inline]
    pub fn get_application_view_download_error_context(
        &self,
        application_id: u64,
        context: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::get_application_view_download_error_context(
            &self.0,
            application_id,
            context,
        )
    }

    #[inline]
    pub fn list_application_content_meta_status(
        &self,
        application_id: u64,
        index: i32,
        list: &mut [u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_application_content_meta_status(
            &self.0,
            types::ContentMetaStatusIn {
                index,
                pad: 0,
                application_id,
            },
            list,
        )
    }

    #[inline]
    pub fn list_download_task_status(&self, tasks: &mut [u8]) -> Result<i32, DispatchError> {
        cmif::app_manager::list_download_task_status(&self.0, tasks)
    }

    #[inline]
    pub fn list_application_id_on_game_card(
        &self,
        application_ids: &mut [u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_application_id_on_game_card(&self.0, application_ids)
    }

    #[inline]
    pub fn get_system_delivery_info(&self, info: &mut [u8]) -> Result<(), DispatchError> {
        cmif::app_manager::get_system_delivery_info(&self.0, info)
    }

    #[inline]
    pub fn verify_delivery_protocol_version(&self, info: &[u8]) -> Result<(), DispatchError> {
        cmif::app_manager::verify_delivery_protocol_version(&self.0, info)
    }

    #[inline]
    pub fn get_application_delivery_info(
        &self,
        info: &mut [u8],
        application_id: u64,
        attr: u32,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::get_application_delivery_info(
            &self.0,
            types::GetApplicationDeliveryInfoIn {
                attr,
                pad: 0,
                application_id,
            },
            info,
        )
    }

    #[inline]
    pub fn has_all_contents_to_deliver(&self, info: &[u8]) -> Result<bool, DispatchError> {
        cmif::app_manager::has_all_contents_to_deliver(&self.0, info)
    }

    #[inline]
    pub fn compare_application_delivery_info(
        &self,
        info0: &[u8],
        info1: &[u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::compare_application_delivery_info(&self.0, info0, info1)
    }

    #[inline]
    pub fn can_deliver_application(
        &self,
        info0: &[u8],
        info1: &[u8],
    ) -> Result<bool, DispatchError> {
        cmif::app_manager::can_deliver_application(&self.0, info0, info1)
    }

    #[inline]
    pub fn list_content_meta_key_to_deliver_application(
        &self,
        meta: &mut [u8],
        meta_index: i32,
        info: &[u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_content_meta_key_to_deliver_application(
            &self.0, meta_index, meta, info,
        )
    }

    #[inline]
    pub fn needs_system_update_to_deliver_application(
        &self,
        sys_info: &[u8],
        app_info: &[u8],
    ) -> Result<bool, DispatchError> {
        cmif::app_manager::needs_system_update_to_deliver_application(&self.0, sys_info, app_info)
    }

    #[inline]
    pub fn estimate_required_size(&self, meta: &[u8]) -> Result<i64, DispatchError> {
        cmif::app_manager::estimate_required_size(&self.0, meta)
    }

    #[inline]
    pub fn compare_system_delivery_info(
        &self,
        info0: &[u8],
        info1: &[u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::compare_system_delivery_info(&self.0, info0, info1)
    }

    #[inline]
    pub fn list_not_committed_content_meta(
        &self,
        meta: &mut [u8],
        application_id: u64,
        unk: i32,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_not_committed_content_meta(
            &self.0,
            types::ListNotCommittedContentMetaIn {
                unk,
                pad: 0,
                application_id,
            },
            meta,
        )
    }

    #[inline]
    pub fn get_application_delivery_info_hash(
        &self,
        info: &[u8],
    ) -> Result<[u8; 0x20], DispatchError> {
        cmif::app_manager::get_application_delivery_info_hash(&self.0, info)
    }

    #[inline]
    pub fn get_application_rights_on_client(
        &self,
        rights: &mut [u8],
        application_id: u64,
        uid: AccountUid,
        flags: u32,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::get_application_rights_on_client(
            &self.0,
            types::GetApplicationRightsOnClientIn {
                flags,
                pad: 0,
                application_id,
                uid,
            },
            rights,
        )
    }

    #[inline]
    pub fn get_promotion_info(
        &self,
        promotion: &mut [u8],
        application_id: &[u8],
        uid: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::get_promotion_info(&self.0, promotion, application_id, uid)
    }

    #[inline]
    pub fn select_latest_system_delivery_info(
        &self,
        base_info: &[u8],
        sys_list: &[u8],
        app_list: &[u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::select_latest_system_delivery_info(
            &self.0, base_info, sys_list, app_list,
        )
    }

    #[inline]
    pub fn estimate_size_to_move(
        &self,
        storage_ids: &[u8],
        storage_id: u8,
        flags: u32,
        application_id: u64,
    ) -> Result<i64, DispatchError> {
        cmif::app_manager::estimate_size_to_move(
            &self.0,
            types::EstimateSizeToMoveIn {
                storage_id,
                pad: [0; 3],
                flags,
                application_id,
            },
            storage_ids,
        )
    }
}

// Async commands

impl NsAppManagerService {
    #[inline]
    pub fn request_application_update_info(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_application_update_info(&self.0, application_id)
    }

    #[inline]
    pub fn request_update_application2(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_update_application2(&self.0, application_id)
    }

    #[inline]
    pub fn request_download_application_control_data(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_download_application_control_data(&self.0, application_id)
    }

    #[inline]
    pub fn request_check_game_card_registration(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_check_game_card_registration(&self.0, application_id)
    }

    #[inline]
    pub fn request_download_application_prepurchased_rights(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_download_application_prepurchased_rights(&self.0, application_id)
    }

    #[inline]
    pub fn request_no_download_rights_error_resolution(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_no_download_rights_error_resolution(&self.0, application_id)
    }

    #[inline]
    pub fn request_resolve_no_download_rights_error(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_resolve_no_download_rights_error(&self.0, application_id)
    }

    #[inline]
    pub fn request_game_card_registration_gold_point(
        &self,
        uid: AccountUid,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_game_card_registration_gold_point(
            &self.0,
            types::GameCardRegistrationGoldPointIn {
                uid,
                application_id,
            },
        )
    }

    #[inline]
    pub fn request_register_game_card(
        &self,
        uid: AccountUid,
        application_id: u64,
        inval: i32,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_register_game_card(
            &self.0,
            types::RegisterGameCardIn {
                inval,
                pad: 0,
                uid,
                application_id,
            },
        )
    }

    #[inline]
    pub fn request_ensure_download_task(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_ensure_download_task(&self.0)
    }

    #[inline]
    pub fn request_download_task_list_data(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_download_task_list_data(&self.0)
    }

    #[inline]
    pub fn request_verify_addon_contents_rights(
        &self,
        application_id: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_verify_addon_contents_rights(&self.0, application_id)
    }

    pub fn request_verify_application_deprecated(
        &self,
        application_id: u64,
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_verify_application_deprecated(
            &self.0,
            types::VerifyApplicationDeprecatedIn {
                application_id,
                tmem_size,
            },
            tmem_handle,
        )
    }

    pub fn request_verify_application(
        &self,
        application_id: u64,
        unk: u32,
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_verify_application(
            &self.0,
            types::VerifyApplicationIn {
                unk,
                pad: 0,
                application_id,
                tmem_size,
            },
            tmem_handle,
        )
    }

    pub fn request_receive_application(
        &self,
        addr: u32,
        port: u16,
        application_id: u64,
        meta: &[u8],
        storage_id: u8,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_receive_application(
            &self.0,
            types::RequestReceiveApplicationIn {
                storage_id,
                pad0: 0,
                port,
                addr,
                application_id,
            },
            meta,
        )
    }

    pub fn request_send_application(
        &self,
        addr: u32,
        port: u16,
        application_id: u64,
        meta: &[u8],
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_send_application(
            &self.0,
            types::RequestSendApplicationIn {
                port,
                pad: 0,
                addr,
                application_id,
            },
            meta,
        )
    }

    pub fn list_application_title(
        &self,
        source: ApplicationControlSource,
        application_ids: &[u8],
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::list_application_title(
            &self.0,
            types::ListApplicationTitleIn {
                source: source as u8,
                pad: [0; 7],
                tmem_size,
            },
            tmem_handle,
            application_ids,
        )
    }

    pub fn list_application_icon(
        &self,
        source: ApplicationControlSource,
        application_ids: &[u8],
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::list_application_icon(
            &self.0,
            types::ListApplicationTitleIn {
                source: source as u8,
                pad: [0; 7],
                tmem_size,
            },
            tmem_handle,
            application_ids,
        )
    }
}

// Sub-object creation

impl NsAppManagerService {
    pub fn get_request_server_stopper(&self) -> Result<NsRequestServerStopper, GetSubObjectError> {
        let raw = cmif::app_manager::get_request_server_stopper(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsRequestServerStopper(Session::new(handle, 0)))
    }

    pub fn delete_user_save_data_all(
        &self,
        uid: AccountUid,
    ) -> Result<NsProgressMonitor, GetSubObjectError> {
        let raw = cmif::app_manager::delete_user_save_data_all(&self.0, uid)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsProgressMonitor(Session::new(handle, 0)))
    }
}

// ===========================================================================
// Sub-interface service types (obtained from getter, thin wrappers)
// ===========================================================================

/// IContentManagementInterface service wrapper.
#[repr(transparent)]
pub struct NsContentManagementService(Session);

impl NsContentManagementService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    pub fn calculate_application_occupied_size(
        &self,
        application_id: u64,
        out: &mut [u8],
    ) -> Result<(), DispatchError> {
        cmif::app_manager::calculate_application_occupied_size(&self.0, application_id, out)
    }

    #[inline]
    pub fn check_sd_card_mount_status(&self) -> Result<(), DispatchError> {
        cmif::app_manager::check_sd_card_mount_status(&self.0)
    }

    #[inline]
    pub fn get_total_space_size(&self, storage_id: u64) -> Result<u64, DispatchError> {
        cmif::app_manager::get_total_space_size(&self.0, storage_id)
    }

    #[inline]
    pub fn get_free_space_size(&self, storage_id: u64) -> Result<u64, DispatchError> {
        cmif::app_manager::get_free_space_size(&self.0, storage_id)
    }

    #[inline]
    pub fn count_application_content_meta(
        &self,
        application_id: u64,
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::count_application_content_meta(&self.0, application_id)
    }

    #[inline]
    pub fn list_application_content_meta_status(
        &self,
        application_id: u64,
        index: i32,
        list: &mut [u8],
    ) -> Result<i32, DispatchError> {
        cmif::app_manager::list_application_content_meta_status(
            &self.0,
            types::ContentMetaStatusIn {
                index,
                pad: 0,
                application_id,
            },
            list,
        )
    }

    #[inline]
    pub fn is_any_application_running(&self) -> Result<bool, DispatchError> {
        cmif::app_manager::is_any_application_running(&self.0)
    }
}

/// IDownloadTaskInterface service wrapper.
#[repr(transparent)]
pub struct NsDownloadTaskService(Session);

impl NsDownloadTaskService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn clear_task_status_list(&self) -> Result<(), DispatchError> {
        cmif::app_manager::clear_task_status_list(&self.0)
    }

    #[inline]
    pub fn request_download_task_list(&self) -> Result<(), DispatchError> {
        cmif::app_manager::request_download_task_list(&self.0)
    }

    #[inline]
    pub fn request_ensure_download_task(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_ensure_download_task(&self.0)
    }

    #[inline]
    pub fn list_download_task_status(&self, tasks: &mut [u8]) -> Result<i32, DispatchError> {
        cmif::app_manager::list_download_task_status(&self.0, tasks)
    }

    #[inline]
    pub fn request_download_task_list_data(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::app_manager::request_download_task_list_data(&self.0)
    }

    #[inline]
    pub fn try_commit_current_application_download_task(&self) -> Result<(), DispatchError> {
        cmif::app_manager::try_commit_current_application_download_task(&self.0)
    }

    #[inline]
    pub fn enable_auto_commit(&self) -> Result<(), DispatchError> {
        cmif::app_manager::enable_auto_commit(&self.0)
    }

    #[inline]
    pub fn disable_auto_commit(&self) -> Result<(), DispatchError> {
        cmif::app_manager::disable_auto_commit(&self.0)
    }

    #[inline]
    pub fn trigger_dynamic_commit_event(&self) -> Result<(), DispatchError> {
        cmif::app_manager::trigger_dynamic_commit_event(&self.0)
    }
}

/// IFactoryResetInterface service wrapper.
#[repr(transparent)]
pub struct NsFactoryResetService(Session);

impl NsFactoryResetService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn reset_to_factory_settings(&self) -> Result<(), DispatchError> {
        cmif::factory_reset::reset_to_factory_settings(&self.0)
    }

    #[inline]
    pub fn reset_to_factory_settings_without_user_save_data(&self) -> Result<(), DispatchError> {
        cmif::factory_reset::reset_to_factory_settings_without_user_save_data(&self.0)
    }

    #[inline]
    pub fn reset_to_factory_settings_for_refurbishment(&self) -> Result<(), DispatchError> {
        cmif::factory_reset::reset_to_factory_settings_for_refurbishment(&self.0)
    }

    #[inline]
    pub fn reset_to_factory_settings_with_platform_region(&self) -> Result<(), DispatchError> {
        cmif::factory_reset::reset_to_factory_settings_with_platform_region(&self.0)
    }

    #[inline]
    pub fn reset_to_factory_settings_with_platform_region_authentication(
        &self,
    ) -> Result<(), DispatchError> {
        cmif::factory_reset::reset_to_factory_settings_with_platform_region_authentication(&self.0)
    }
}

/// IECommerceInterface service wrapper.
#[repr(transparent)]
pub struct NsECommerceService(Session);

impl NsECommerceService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn request_link_device(&self, uid: AccountUid) -> Result<AsyncOut, AsyncCommandError> {
        cmif::ecommerce::request_link_device(&self.0, uid)
    }

    #[inline]
    pub fn request_sync_rights(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::ecommerce::request_sync_rights(&self.0)
    }

    #[inline]
    pub fn request_unlink_device(&self, uid: AccountUid) -> Result<AsyncOut, AsyncCommandError> {
        cmif::ecommerce::request_unlink_device(&self.0, uid)
    }
}

/// IReadOnlyApplicationControlDataInterface service wrapper.
#[repr(transparent)]
pub struct NsReadOnlyControlDataService(Session);

impl NsReadOnlyControlDataService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    pub fn get_application_control_data(
        &self,
        source: ApplicationControlSource,
        application_id: u64,
        buffer: &mut [u8],
    ) -> Result<u32, DispatchError> {
        cmif::control_data::get_application_control_data(
            &self.0,
            types::ControlDataSourceAppIdIn {
                source: source as u8,
                pad: [0; 7],
                application_id,
            },
            buffer,
        )
    }

    #[inline]
    pub fn get_application_desired_language(&self, lang_bitmask: u8) -> Result<u8, DispatchError> {
        cmif::control_data::get_application_desired_language(&self.0, lang_bitmask)
    }

    pub fn get_application_control_data2(
        &self,
        source: ApplicationControlSource,
        application_id: u64,
        buffer: &mut [u8],
        flag1: u8,
        acd_idx: u8,
    ) -> Result<u64, DispatchError> {
        cmif::control_data::get_application_control_data2(
            &self.0,
            types::ControlData2In {
                source: source as u8,
                flag1,
                acd_idx,
                pad: [0; 5],
                application_id,
            },
            buffer,
        )
    }

    pub fn list_application_title2(
        &self,
        source: u8,
        application_ids: &[u8],
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::control_data::list_application_title2(
            &self.0,
            types::ListApplicationTitleIn {
                source,
                pad: [0; 7],
                tmem_size,
            },
            tmem_handle,
            application_ids,
        )
    }
}

/// IReadOnlyApplicationRecordInterface service wrapper (5.0.0+, opaque).
#[repr(transparent)]
pub struct NsReadOnlyRecordService(Session);

impl NsReadOnlyRecordService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IDynamicRightsInterface service wrapper (6.0.0+, opaque).
#[repr(transparent)]
pub struct NsDynamicRightsService(Session);

impl NsDynamicRightsService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IApplicationVersionInterface service wrapper (4.0.0+, opaque).
#[repr(transparent)]
pub struct NsApplicationVersionService(Session);

impl NsApplicationVersionService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IAccountProxyInterface service wrapper (3.0.0+, opaque).
#[repr(transparent)]
pub struct NsAccountProxyService(Session);

impl NsAccountProxyService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IDocumentInterface service wrapper (3.0.0+, opaque).
#[repr(transparent)]
pub struct NsDocumentService(Session);

impl NsDocumentService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

// ===========================================================================
// Sub-objects
// ===========================================================================

/// IRequestServerStopper sub-object.
#[repr(transparent)]
pub struct NsRequestServerStopper(Session);

impl NsRequestServerStopper {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// IProgressMonitorForDeleteUserSaveDataAll sub-object.
#[repr(transparent)]
pub struct NsProgressMonitor(Session);

impl NsProgressMonitor {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn get_system_event(&self) -> Result<u32, SubObjectAcquireEventError> {
        cmif::sub_objects::progress_monitor_get_system_event(&self.0)
    }

    #[inline]
    pub fn is_finished(&self) -> Result<bool, DispatchError> {
        cmif::sub_objects::progress_monitor_is_finished(&self.0)
    }

    #[inline]
    pub fn get_result(&self) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_monitor_get_result(&self.0)
    }

    #[inline]
    pub fn get_progress(&self) -> Result<ProgressForDeleteUserSaveDataAll, DispatchError> {
        cmif::sub_objects::progress_monitor_get_progress(&self.0)
    }
}

/// IProgressAsyncResult sub-object.
#[repr(transparent)]
pub struct NsProgressAsyncResult(Session);

impl NsProgressAsyncResult {
    /// Adopts a pre-obtained IProgressAsyncResult session handle.
    ///
    /// The caller must ensure `raw` names a live IProgressAsyncResult session this process
    /// owns and that nothing else will close, since the returned value closes it on drop.
    /// A second owner closes a handle number the kernel may have reused, which tears down an
    /// unrelated session rather than faulting.
    pub fn from_raw_unchecked(raw: u32) -> Self {
        // SAFETY: Delegated to this constructor's precondition, which is where the caller
        // vouches for a handle ns handed it back on an async command.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Self(Session::new(handle, 0))
    }

    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn get(&self) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_async_get(&self.0)
    }

    #[inline]
    pub fn cancel(&self) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_async_cancel(&self.0)
    }

    #[inline]
    pub fn get_progress(&self, buffer: &mut [u8]) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_async_get_progress(&self.0, buffer)
    }

    #[inline]
    pub fn get_detail_result(&self) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_async_get_detail_result(&self.0)
    }

    #[inline]
    pub fn get_error_context(&self, context: &mut [u8]) -> Result<(), DispatchError> {
        cmif::sub_objects::progress_async_get_error_context(&self.0, context)
    }
}

// ===========================================================================
// NsvmService — ns:vm
// ===========================================================================

/// ns:vm service wrapper.
#[repr(transparent)]
pub struct NsvmService(Session);

impl NsvmService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn needs_update_vulnerability(&self) -> Result<bool, DispatchError> {
        cmif::nsvm::needs_update_vulnerability(&self.0)
    }

    #[inline]
    pub fn get_safe_system_version(&self) -> Result<NcmContentMetaKey, DispatchError> {
        cmif::nsvm::get_safe_system_version(&self.0)
    }
}

pub fn connect_nsvm_cmif(sm: &SmService) -> Result<NsvmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(NSVM_SERVICE_NAME)
        .map_err(ConnectCmifError)?;
    Ok(NsvmService(Session::new(handle, 0)))
}

// ===========================================================================
// NsdevService — ns:dev
// ===========================================================================

/// ns:dev service wrapper.
#[repr(transparent)]
pub struct NsdevService(Session);

impl NsdevService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn launch_program(
        &self,
        properties: &LaunchProperties,
        flags: u32,
    ) -> Result<u64, DispatchError> {
        cmif::nsdev::launch_program(
            &self.0,
            types::NsdevLaunchProgramIn {
                flags,
                pad: 0,
                properties: *properties,
                pad2: 0,
            },
        )
    }

    #[inline]
    pub fn terminate_process(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::nsdev::terminate_process(&self.0, pid)
    }

    #[inline]
    pub fn terminate_program(&self, tid: u64) -> Result<(), DispatchError> {
        cmif::nsdev::terminate_program(&self.0, tid)
    }

    #[inline]
    pub fn get_shell_event(&self) -> Result<u32, NsdevAcquireEventError> {
        cmif::nsdev::get_shell_event(&self.0)
    }

    #[inline]
    pub fn get_shell_event_info(&self) -> Result<ShellEventInfo, DispatchError> {
        cmif::nsdev::get_shell_event_info(&self.0)
    }

    #[inline]
    pub fn terminate_application(&self) -> Result<(), DispatchError> {
        cmif::nsdev::terminate_application(&self.0)
    }

    #[inline]
    pub fn prepare_launch_program_from_host(
        &self,
        path: &[u8],
    ) -> Result<LaunchProperties, DispatchError> {
        cmif::nsdev::prepare_launch_program_from_host(&self.0, path)
    }

    #[inline]
    pub fn launch_application_for_develop(
        &self,
        application_id: u64,
        flags: u32,
    ) -> Result<u64, DispatchError> {
        cmif::nsdev::launch_application_for_develop(
            &self.0,
            types::NsdevLaunchApplicationForDevelopIn {
                flags,
                pad: 0,
                application_id,
            },
        )
    }

    #[inline]
    pub fn launch_application_from_host(
        &self,
        path: &[u8],
        flags: u32,
    ) -> Result<u64, DispatchError> {
        cmif::nsdev::launch_application_from_host(&self.0, flags, path)
    }

    #[inline]
    pub fn launch_application_with_storage_id_for_develop(
        &self,
        application_id: u64,
        flags: u32,
        app_storage_id: u8,
        patch_storage_id: u8,
    ) -> Result<u64, DispatchError> {
        cmif::nsdev::launch_application_with_storage_id_for_develop(
            &self.0,
            types::NsdevLaunchApplicationWithStorageIdIn {
                app_storage_id,
                patch_storage_id,
                pad: [0; 2],
                flags,
                application_id,
            },
        )
    }

    #[inline]
    pub fn is_system_memory_resource_limit_boosted(&self) -> Result<bool, DispatchError> {
        cmif::nsdev::is_system_memory_resource_limit_boosted(&self.0)
    }

    #[inline]
    pub fn get_running_application_process_id_for_develop(&self) -> Result<u64, DispatchError> {
        cmif::nsdev::get_running_application_process_id_for_develop(&self.0)
    }

    #[inline]
    pub fn set_current_application_rights_environment_can_be_active(
        &self,
        can_be_active: bool,
    ) -> Result<(), DispatchError> {
        cmif::nsdev::set_current_application_rights_environment_can_be_active(
            &self.0,
            can_be_active as u8,
        )
    }
}

pub fn connect_nsdev_cmif(sm: &SmService) -> Result<NsdevService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(NSDEV_SERVICE_NAME)
        .map_err(ConnectCmifError)?;
    Ok(NsdevService(Session::new(handle, 0)))
}

// ===========================================================================
// NssuService — ns:su
// ===========================================================================

/// ns:su service wrapper.
#[repr(transparent)]
pub struct NssuService(Session);

impl NssuService {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn get_background_network_update_state(&self) -> Result<u8, DispatchError> {
        cmif::nssu::get_background_network_update_state(&self.0)
    }

    pub fn open_system_update_control(
        &self,
    ) -> Result<NsSystemUpdateControl, OpenSystemUpdateControlError> {
        let raw = cmif::nssu::open_system_update_control(&self.0)?;
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(Handle::from_raw_unchecked(raw));
        Ok(NsSystemUpdateControl(Session::new(handle, 0)))
    }

    #[inline]
    pub fn notify_exfat_driver_required(&self) -> Result<(), DispatchError> {
        cmif::nssu::notify_exfat_driver_required(&self.0)
    }

    #[inline]
    pub fn clear_exfat_driver_status_for_debug(&self) -> Result<(), DispatchError> {
        cmif::nssu::clear_exfat_driver_status_for_debug(&self.0)
    }

    #[inline]
    pub fn request_background_network_update(&self) -> Result<(), DispatchError> {
        cmif::nssu::request_background_network_update(&self.0)
    }

    #[inline]
    pub fn notify_background_network_update(
        &self,
        key: NcmContentMetaKey,
    ) -> Result<(), DispatchError> {
        cmif::nssu::notify_background_network_update(&self.0, key)
    }

    #[inline]
    pub fn notify_exfat_driver_downloaded_for_debug(&self) -> Result<(), DispatchError> {
        cmif::nssu::notify_exfat_driver_downloaded_for_debug(&self.0)
    }

    #[inline]
    pub fn get_system_update_notification_event(&self) -> Result<u32, NssuAcquireEventError> {
        cmif::nssu::get_system_update_notification_event(&self.0)
    }

    #[inline]
    pub fn notify_system_update_for_content_delivery(&self) -> Result<(), DispatchError> {
        cmif::nssu::notify_system_update_for_content_delivery(&self.0)
    }

    #[inline]
    pub fn prepare_shutdown(&self) -> Result<(), DispatchError> {
        cmif::nssu::prepare_shutdown(&self.0)
    }

    #[inline]
    pub fn destroy_system_update_task(&self) -> Result<(), DispatchError> {
        cmif::nssu::destroy_system_update_task(&self.0)
    }

    #[inline]
    pub fn request_send_system_update(
        &self,
        addr: u32,
        port: u16,
        info: &[u8],
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::request_send_system_update(
            &self.0,
            types::RequestSendReceiveSystemUpdateIn { port, pad: 0, addr },
            info,
        )
    }

    #[inline]
    pub fn get_send_system_update_progress(&self) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::nssu::get_send_system_update_progress(&self.0)
    }
}

pub fn connect_nssu_cmif(sm: &SmService) -> Result<NssuService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(NSSU_SERVICE_NAME)
        .map_err(ConnectCmifError)?;
    Ok(NssuService(Session::new(handle, 0)))
}

// ===========================================================================
// NsSystemUpdateControl — ISystemUpdateControl sub-object (from ns:su)
// ===========================================================================

/// ISystemUpdateControl sub-object wrapper.
#[repr(transparent)]
pub struct NsSystemUpdateControl(Session);

impl NsSystemUpdateControl {
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    #[inline]
    pub fn has_downloaded(&self) -> Result<bool, DispatchError> {
        cmif::nssu::ctrl_has_downloaded(&self.0)
    }

    #[inline]
    pub fn request_check_latest_update(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::ctrl_request_check_latest_update(&self.0)
    }

    #[inline]
    pub fn request_download_latest_update(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::ctrl_request_download_latest_update(&self.0)
    }

    #[inline]
    pub fn get_download_progress(&self) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::nssu::ctrl_get_download_progress(&self.0)
    }

    #[inline]
    pub fn apply_downloaded_update(&self) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_apply_downloaded_update(&self.0)
    }

    #[inline]
    pub fn request_prepare_card_update(&self) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::ctrl_request_prepare_card_update(&self.0)
    }

    #[inline]
    pub fn get_prepare_card_update_progress(&self) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::nssu::ctrl_get_prepare_card_update_progress(&self.0)
    }

    #[inline]
    pub fn has_prepared_card_update(&self) -> Result<bool, DispatchError> {
        cmif::nssu::ctrl_has_prepared_card_update(&self.0)
    }

    #[inline]
    pub fn apply_card_update(&self) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_apply_card_update(&self.0)
    }

    #[inline]
    pub fn get_downloaded_eula_data_size(&self, path: &[u8]) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_downloaded_eula_data_size(&self.0, path)
    }

    #[inline]
    pub fn get_downloaded_eula_data(
        &self,
        path: &[u8],
        buffer: &mut [u8],
    ) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_downloaded_eula_data(&self.0, path, buffer)
    }

    #[inline]
    pub fn setup_card_update(&self, tmem_handle: u32, tmem_size: u64) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_setup_card_update(&self.0, tmem_size, tmem_handle)
    }

    #[inline]
    pub fn get_prepared_card_update_eula_data_size(
        &self,
        path: &[u8],
    ) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_prepared_card_update_eula_data_size(&self.0, path)
    }

    #[inline]
    pub fn get_prepared_card_update_eula_data(
        &self,
        path: &[u8],
        buffer: &mut [u8],
    ) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_prepared_card_update_eula_data(&self.0, path, buffer)
    }

    #[inline]
    pub fn setup_card_update_via_system_updater(
        &self,
        tmem_handle: u32,
        tmem_size: u64,
    ) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_setup_card_update_via_system_updater(&self.0, tmem_size, tmem_handle)
    }

    #[inline]
    pub fn has_received(&self) -> Result<bool, DispatchError> {
        cmif::nssu::ctrl_has_received(&self.0)
    }

    #[inline]
    pub fn request_receive_system_update(
        &self,
        addr: u32,
        port: u16,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::ctrl_request_receive_system_update(
            &self.0,
            types::RequestSendReceiveSystemUpdateIn { port, pad: 0, addr },
        )
    }

    #[inline]
    pub fn get_receive_progress(&self) -> Result<SystemUpdateProgress, DispatchError> {
        cmif::nssu::ctrl_get_receive_progress(&self.0)
    }

    #[inline]
    pub fn apply_received_update(&self) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_apply_received_update(&self.0)
    }

    #[inline]
    pub fn get_received_eula_data_size(&self, path: &[u8]) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_received_eula_data_size(&self.0, path)
    }

    #[inline]
    pub fn get_received_eula_data(
        &self,
        path: &[u8],
        buffer: &mut [u8],
    ) -> Result<u64, DispatchError> {
        cmif::nssu::ctrl_get_received_eula_data(&self.0, path, buffer)
    }

    #[inline]
    pub fn setup_to_receive_system_update(&self) -> Result<(), DispatchError> {
        cmif::nssu::ctrl_setup_to_receive_system_update(&self.0)
    }

    #[inline]
    pub fn request_check_latest_update_includes_rebootless_update(
        &self,
    ) -> Result<AsyncOut, AsyncCommandError> {
        cmif::nssu::ctrl_request_check_latest_update_includes_rebootless_update(&self.0)
    }
}
