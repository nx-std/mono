//! News service (`news:a`/`news:c`/`news:m`/`news:p`/`news:v`) implementation.
//!
//! Provides access to the news/notification feed system for reading,
//! posting, and managing news articles and subscriptions.
//!
//! ## Architecture
//!
//! The service is non-domain. On 2.0.0+ the service uses a two-step connect:
//! first connect to the creator service, then create the news service
//! sub-object. On pre-2.0.0 the service handle is used directly.
//!
//! Sub-objects ([`NewsNewlyArrivedEventHolder`], [`NewsDataService`],
//! [`NewsDatabaseService`], [`NewsOverwriteEventHolder`]) are obtained from
//! either the creator session (2.0.0+) or the news service session (pre-2.0.0)
//! using different command IDs.
//!
//! ## Divergence from libnx
//!
//! libnx's `news.c` uses hosversion checks and a global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`. This crate is hosversion-unaware per IC-4:
//! it exposes separate `connect_cmif` (for 2.0.0+) and `connect_cmif_legacy`
//! (for pre-2.0.0) functions. Sub-object creation methods are similarly paired
//! with `_legacy` variants that dispatch on the news service session with
//! different command IDs.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        DispatchError,
        OwnedSessionHandle,
        Session,
    },
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{
        CreateSubObjectError,
        EventHolderGetError,
    },
    proto::{
        SERVICE_NAME_ADMIN,
        SERVICE_NAME_CONFIG,
        SERVICE_NAME_MANAGER,
        SERVICE_NAME_POST,
        SERVICE_NAME_VIEWER,
    },
    types::{
        NewsRecord,
        NewsRecordV1,
        NewsTopicName,
    },
};

/// News service type selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsServiceType {
    Administrator,
    Configuration,
    Manager,
    Post,
    Viewer,
}

impl NewsServiceType {
    fn service_name(self) -> nx_sf::ServiceName {
        match self {
            Self::Administrator => SERVICE_NAME_ADMIN,
            Self::Configuration => SERVICE_NAME_CONFIG,
            Self::Manager => SERVICE_NAME_MANAGER,
            Self::Post => SERVICE_NAME_POST,
            Self::Viewer => SERVICE_NAME_VIEWER,
        }
    }
}

/// Connected news creator + news service wrapper (2.0.0+ connection pattern).
///
/// Holds both the creator session (for creating sub-objects) and the news
/// service session (for news commands).
pub struct NewsService {
    creator: Session,
    service: Session,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NewsService {}
unsafe impl Sync for NewsService {}

impl NewsService {
    /// Posts local news data.
    #[inline]
    pub fn post_local_news(&self, news: &[u8]) -> Result<(), DispatchError> {
        cmif::post_local_news(&self.service, news)
    }

    /// Sets the passphrase for a program.
    ///
    /// `passphrase` should include the null terminator.
    #[inline]
    pub fn set_passphrase(&self, program_id: u64, passphrase: &[u8]) -> Result<(), DispatchError> {
        cmif::set_passphrase(&self.service, program_id, passphrase)
    }

    /// Gets the subscription status for a filter string.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn get_subscription_status(&self, filter: &[u8]) -> Result<u32, DispatchError> {
        cmif::get_subscription_status(&self.service, filter)
    }

    /// Gets the topic list for a channel. \[3.0.0+\]
    ///
    /// `out_buf` should be sized as `max_count * size_of::<NewsTopicName>()`.
    /// Returns the number of topics written.
    #[inline]
    pub fn get_topic_list(&self, channel: u32, out_buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_topic_list(&self.service, channel, out_buf)
    }

    /// Gets save data usage statistics. \[6.0.0+\]
    ///
    /// Returns `(current_bytes, total_bytes)`.
    pub fn get_savedata_usage(&self) -> Result<(u64, u64), DispatchError> {
        let out = cmif::get_savedata_usage(&self.service)?;
        Ok((out.current, out.total))
    }

    /// Checks if a system update is required.
    #[inline]
    pub fn is_system_update_required(&self) -> Result<bool, DispatchError> {
        cmif::is_system_update_required(&self.service)
    }

    /// Gets the database version. \[10.0.0+\]
    #[inline]
    pub fn get_database_version(&self) -> Result<u32, DispatchError> {
        cmif::get_database_version(&self.service)
    }

    /// Requests immediate reception with a filter.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn request_immediate_reception(&self, filter: &[u8]) -> Result<(), DispatchError> {
        cmif::request_immediate_reception(&self.service, filter)
    }

    /// Sets the subscription status for a filter.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn set_subscription_status(&self, filter: &[u8], status: u32) -> Result<(), DispatchError> {
        cmif::set_subscription_status(&self.service, status, filter)
    }

    /// Clears all news storage.
    #[inline]
    pub fn clear_storage(&self) -> Result<(), DispatchError> {
        cmif::clear_storage(&self.service)
    }

    /// Clears all subscription statuses.
    #[inline]
    pub fn clear_subscription_status_all(&self) -> Result<(), DispatchError> {
        cmif::clear_subscription_status_all(&self.service)
    }

    /// Dumps the news database into the provided buffer.
    ///
    /// Returns the number of bytes written.
    #[inline]
    pub fn get_news_database_dump(&self, buffer: &mut [u8]) -> Result<u64, DispatchError> {
        cmif::get_news_database_dump(&self.service, buffer)
    }

    /// Creates a newly-arrived event holder sub-object (2.0.0+ pattern).
    pub fn create_newly_arrived_event_holder(
        &self,
    ) -> Result<NewsNewlyArrivedEventHolder, CreateSubObjectError> {
        let handle =
            cmif::create_sub_object(&self.creator, proto::CREATE_NEWLY_ARRIVED_EVENT_HOLDER)?;
        Ok(NewsNewlyArrivedEventHolder(make_service(handle)))
    }

    /// Creates a news data service sub-object (2.0.0+ pattern).
    pub fn create_news_data_service(&self) -> Result<NewsDataService, CreateSubObjectError> {
        let handle = cmif::create_sub_object(&self.creator, proto::CREATE_NEWS_DATA_SERVICE)?;
        Ok(NewsDataService(make_service(handle)))
    }

    /// Creates a news database service sub-object (2.0.0+ pattern).
    pub fn create_news_database_service(
        &self,
    ) -> Result<NewsDatabaseService, CreateSubObjectError> {
        let handle = cmif::create_sub_object(&self.creator, proto::CREATE_NEWS_DATABASE_SERVICE)?;
        Ok(NewsDatabaseService(make_service(handle)))
    }

    /// Creates an overwrite event holder sub-object (2.0.0+).
    pub fn create_overwrite_event_holder(
        &self,
    ) -> Result<NewsOverwriteEventHolder, CreateSubObjectError> {
        let handle = cmif::create_sub_object(&self.creator, proto::CREATE_OVERWRITE_EVENT_HOLDER)?;
        Ok(NewsOverwriteEventHolder(make_service(handle)))
    }
}

/// Connected news service wrapper (pre-2.0.0 legacy connection pattern).
///
/// On pre-2.0.0 firmware, the SM lookup returns the news service directly.
/// Sub-objects are created from the service session with different command IDs.
pub struct NewsServiceLegacy {
    service: Session,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for NewsServiceLegacy {}
unsafe impl Sync for NewsServiceLegacy {}

impl NewsServiceLegacy {
    /// Posts local news data.
    #[inline]
    pub fn post_local_news(&self, news: &[u8]) -> Result<(), DispatchError> {
        cmif::post_local_news(&self.service, news)
    }

    /// Sets the passphrase for a program.
    ///
    /// `passphrase` should include the null terminator.
    #[inline]
    pub fn set_passphrase(&self, program_id: u64, passphrase: &[u8]) -> Result<(), DispatchError> {
        cmif::set_passphrase(&self.service, program_id, passphrase)
    }

    /// Gets the subscription status for a filter string.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn get_subscription_status(&self, filter: &[u8]) -> Result<u32, DispatchError> {
        cmif::get_subscription_status(&self.service, filter)
    }

    /// Checks if a system update is required.
    #[inline]
    pub fn is_system_update_required(&self) -> Result<bool, DispatchError> {
        cmif::is_system_update_required(&self.service)
    }

    /// Requests immediate reception with a filter.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn request_immediate_reception(&self, filter: &[u8]) -> Result<(), DispatchError> {
        cmif::request_immediate_reception(&self.service, filter)
    }

    /// Sets the subscription status for a filter.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn set_subscription_status(&self, filter: &[u8], status: u32) -> Result<(), DispatchError> {
        cmif::set_subscription_status(&self.service, status, filter)
    }

    /// Clears all news storage.
    #[inline]
    pub fn clear_storage(&self) -> Result<(), DispatchError> {
        cmif::clear_storage(&self.service)
    }

    /// Clears all subscription statuses.
    #[inline]
    pub fn clear_subscription_status_all(&self) -> Result<(), DispatchError> {
        cmif::clear_subscription_status_all(&self.service)
    }

    /// Dumps the news database into the provided buffer.
    ///
    /// Returns the number of bytes written.
    #[inline]
    pub fn get_news_database_dump(&self, buffer: &mut [u8]) -> Result<u64, DispatchError> {
        cmif::get_news_database_dump(&self.service, buffer)
    }

    /// Creates a newly-arrived event holder sub-object (pre-2.0.0 pattern).
    pub fn create_newly_arrived_event_holder(
        &self,
    ) -> Result<NewsNewlyArrivedEventHolder, CreateSubObjectError> {
        let handle = cmif::create_sub_object(
            &self.service,
            proto::CREATE_NEWLY_ARRIVED_EVENT_HOLDER_LEGACY,
        )?;
        Ok(NewsNewlyArrivedEventHolder(make_service(handle)))
    }

    /// Creates a news data service sub-object (pre-2.0.0 pattern).
    pub fn create_news_data_service(&self) -> Result<NewsDataService, CreateSubObjectError> {
        let handle =
            cmif::create_sub_object(&self.service, proto::CREATE_NEWS_DATA_SERVICE_LEGACY)?;
        Ok(NewsDataService(make_service(handle)))
    }

    /// Creates a news database service sub-object (pre-2.0.0 pattern).
    pub fn create_news_database_service(
        &self,
    ) -> Result<NewsDatabaseService, CreateSubObjectError> {
        let handle =
            cmif::create_sub_object(&self.service, proto::CREATE_NEWS_DATABASE_SERVICE_LEGACY)?;
        Ok(NewsDatabaseService(make_service(handle)))
    }
}

// ---------------------------------------------------------------------------
// Sub-objects
// ---------------------------------------------------------------------------

/// Newly-arrived event holder sub-object.
pub struct NewsNewlyArrivedEventHolder(Session);

impl NewsNewlyArrivedEventHolder {
    /// Gets the event handle for newly-arrived news notifications.
    ///
    /// The returned handle is a copy handle for a readable event.
    #[inline]
    pub fn get(&self) -> Result<u32, EventHolderGetError> {
        cmif::event_holder_get(&self.0)
    }
}

/// News data service sub-object.
pub struct NewsDataService(Session);

impl NewsDataService {
    /// Opens news data by file name.
    ///
    /// `file_name` should include the null terminator.
    #[inline]
    pub fn open(&self, file_name: &[u8]) -> Result<(), DispatchError> {
        cmif::data_open(&self.0, file_name)
    }

    /// Opens news data with a V1 record (pre-6.0.0).
    #[inline]
    pub fn open_with_record_v1(&self, record: &NewsRecordV1) -> Result<(), DispatchError> {
        cmif::data_open_with_record_v1(&self.0, record)
    }

    /// Reads data from the currently-opened news data.
    ///
    /// Returns the number of bytes read.
    #[inline]
    pub fn read(&self, offset: u64, out: &mut [u8]) -> Result<u64, DispatchError> {
        cmif::data_read(&self.0, offset, out)
    }

    /// Gets the size of the currently-opened news data.
    #[inline]
    pub fn get_size(&self) -> Result<u64, DispatchError> {
        cmif::data_get_size(&self.0)
    }

    /// Opens news data with a current record. \[6.0.0+\]
    #[inline]
    pub fn open_with_record(&self, record: &NewsRecord) -> Result<(), DispatchError> {
        cmif::data_open_with_record(&self.0, record)
    }
}

/// News database service sub-object.
pub struct NewsDatabaseService(Session);

impl NewsDatabaseService {
    /// Gets a list of V1 records (pre-6.0.0).
    ///
    /// `out_buf` should be sized as `max_count * size_of::<NewsRecordV1>()`.
    /// `where_clause` and `order_clause` should include the null terminator.
    /// Returns the number of records written.
    #[inline]
    pub fn get_list_v1(
        &self,
        offset: u32,
        out_buf: &mut [u8],
        where_clause: &[u8],
        order_clause: &[u8],
    ) -> Result<u32, DispatchError> {
        cmif::database_get_list_v1(&self.0, offset, out_buf, where_clause, order_clause)
    }

    /// Counts records matching a filter.
    ///
    /// `filter` should include the null terminator.
    #[inline]
    pub fn count(&self, filter: &[u8]) -> Result<u32, DispatchError> {
        cmif::database_count(&self.0, filter)
    }

    /// Gets a list of current records. \[6.0.0+\]
    ///
    /// `out_buf` should be sized as `max_count * size_of::<NewsRecord>()`.
    /// `where_clause` and `order_clause` should include the null terminator.
    /// Returns the number of records written.
    #[inline]
    pub fn get_list(
        &self,
        offset: u32,
        out_buf: &mut [u8],
        where_clause: &[u8],
        order_clause: &[u8],
    ) -> Result<u32, DispatchError> {
        cmif::database_get_list(&self.0, offset, out_buf, where_clause, order_clause)
    }
}

/// Overwrite event holder sub-object (2.0.0+).
pub struct NewsOverwriteEventHolder(Session);

impl NewsOverwriteEventHolder {
    /// Gets the event handle for overwrite notifications.
    ///
    /// The returned handle is a copy handle for a readable event.
    #[inline]
    pub fn get(&self) -> Result<u32, EventHolderGetError> {
        cmif::event_holder_get(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Constructs a `Session` from a raw move handle.
fn make_service(raw_handle: u32) -> Session {
    // SAFETY: the kernel returned a valid move handle in the IPC response,
    // transferring ownership of the session to this process.
    let handle =
        OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(raw_handle));
    Session::new(handle, 0)
}

// ---------------------------------------------------------------------------
// Connect functions
// ---------------------------------------------------------------------------

/// Connects to a news service using CMIF (2.0.0+ pattern).
///
/// Obtains the creator session via SM, then creates the news service
/// sub-object from the creator.
pub fn connect_cmif(
    sm: &SmService,
    service_type: NewsServiceType,
) -> Result<NewsService, ConnectCmifError> {
    let session = sm
        .get_service_handle_cmif(service_type.service_name())
        .map_err(ConnectCmifError::GetService)?;

    let creator = Session::new(session, 0);

    let service = match cmif::create_news_service(&creator) {
        Ok(handle) => make_service(handle),
        Err(err) => return Err(ConnectCmifError::CreateNewsService(err)),
    };

    Ok(NewsService { creator, service })
}

/// Connects to a news service using CMIF (pre-2.0.0 legacy pattern).
///
/// On pre-2.0.0 the SM lookup returns the news service directly; there
/// is no creator session.
pub fn connect_cmif_legacy(
    sm: &SmService,
    service_type: NewsServiceType,
) -> Result<NewsServiceLegacy, ConnectCmifLegacyError> {
    let session = sm
        .get_service_handle_cmif(service_type.service_name())
        .map_err(ConnectCmifLegacyError::GetService)?;

    let service = Session::new(session, 0);

    Ok(NewsServiceLegacy { service })
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for the news service failed.
    #[error("failed to look up news service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Creating the news service sub-object from the creator failed.
    #[error("failed to create news service sub-object from creator")]
    CreateNewsService(#[source] CreateSubObjectError),
}

/// Errors returned by [`connect_cmif_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifLegacyError {
    /// SM lookup for the news service failed.
    #[error("failed to look up news service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
