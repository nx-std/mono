#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, Session, clone_current_object};

mod cmif;
mod dispatch;
mod proto;
mod session;
pub mod types;

use self::session::SessionPool;
pub use self::{proto::SERVICE_NAME, types::*};

// ---------------------------------------------------------------------------
// FsContext — shared state for the root service and all sub-objects
// ---------------------------------------------------------------------------

struct FsContext {
    pool: SessionPool,
    priority: AtomicU32,
}

impl FsContext {
    fn ctx(&self) -> u32 {
        self.priority.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// FsService (root — IFileSystemProxy)
// ---------------------------------------------------------------------------

pub struct FsService {
    inner: FsContext,
}

unsafe impl Send for FsService {}
unsafe impl Sync for FsService {}

impl FsService {
    pub fn set_priority(&self, priority: Priority) {
        self.inner
            .priority
            .store(priority as u32, Ordering::Relaxed);
    }

    pub fn open_file_system_legacy(
        &self,
        fs_type: FileSystemType,
        content_path: &[u8; FS_MAX_PATH],
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_file_system_legacy(
            guard.domain(),
            self.inner.ctx(),
            fs_type as u32,
            content_path,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_data_file_system_by_current_process(
        &self,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_data_file_system_by_current_process(
            guard.domain(),
            self.inner.ctx(),
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_file_system_with_patch(
        &self,
        id: u64,
        fs_type: FileSystemType,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_file_system_with_patch(
            guard.domain(),
            self.inner.ctx(),
            id,
            fs_type as u32,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_file_system_with_id(
        &self,
        id: u64,
        fs_type: FileSystemType,
        content_path: &[u8; FS_MAX_PATH],
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_file_system_with_id(
            guard.domain(),
            self.inner.ctx(),
            id,
            fs_type as u32,
            content_path,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_file_system_with_id_v16(
        &self,
        id: u64,
        fs_type: FileSystemType,
        attr: ContentAttributes,
        content_path: &[u8; FS_MAX_PATH],
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_file_system_with_id_v16(
            guard.domain(),
            self.inner.ctx(),
            id,
            fs_type as u32,
            attr as u8,
            content_path,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_data_file_system_by_program_id(
        &self,
        program_id: u64,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_data_file_system_by_program_id(
            guard.domain(),
            self.inner.ctx(),
            program_id,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_bis_file_system(
        &self,
        partition_id: BisPartitionId,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_bis_file_system(
            guard.domain(),
            self.inner.ctx(),
            partition_id as u32,
            path,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_bis_storage(
        &self,
        partition_id: BisPartitionId,
    ) -> Result<FsStorage<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw =
            cmif::proxy::open_bis_storage(guard.domain(), self.inner.ctx(), partition_id as u32)?;
        Ok(FsStorage {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_sd_card_file_system(&self) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_sd_card_file_system(guard.domain(), self.inner.ctx())?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_host_file_system(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_host_file_system(guard.domain(), self.inner.ctx(), path)?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_host_file_system_with_option(
        &self,
        path: &[u8; FS_MAX_PATH],
        flags: MountHostOption,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_host_file_system_with_option(
            guard.domain(),
            self.inner.ctx(),
            path,
            flags.bits(),
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn delete_save_data_file_system(&self, application_id: u64) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::delete_save_data_file_system(guard.domain(), self.inner.ctx(), application_id)
    }

    pub fn create_save_data_file_system(
        &self,
        attr: &SaveDataAttribute,
        creation_info: &SaveDataCreationInfo,
        meta: &SaveDataMetaInfo,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = CreateSaveDataIn {
            attr: *attr,
            creation_info: *creation_info,
            meta: *meta,
        };
        cmif::proxy::create_save_data_file_system_raw(guard.domain(), self.inner.ctx(), &input)
    }

    pub fn create_save_data_file_system_by_system_save_data_id(
        &self,
        attr: &SaveDataAttribute,
        creation_info: &SaveDataCreationInfo,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = CreateSaveDataBySystemIdIn {
            attr: *attr,
            creation_info: *creation_info,
        };
        cmif::proxy::create_save_data_file_system_by_system_save_data_id(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )
    }

    pub fn delete_save_data_file_system_by_save_data_space_id(
        &self,
        space_id: SaveDataSpaceId,
        save_id: u64,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = DeleteSaveDataBySpaceIdIn {
            save_data_space_id: space_id as u8,
            _pad: [0; 7],
            save_id,
        };
        cmif::proxy::delete_save_data_file_system_by_save_data_space_id(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )
    }

    pub fn delete_save_data_file_system_by_save_data_attribute(
        &self,
        space_id: SaveDataSpaceId,
        attr: &SaveDataAttribute,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = DeleteSaveDataByAttributeIn {
            save_data_space_id: space_id as u8,
            _pad: [0; 7],
            attr: *attr,
        };
        cmif::proxy::delete_save_data_file_system_by_save_data_attribute(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )
    }

    pub fn is_exfat_supported(&self) -> Result<bool, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::is_exfat_supported(guard.domain(), self.inner.ctx())
    }

    pub fn open_game_card_file_system(
        &self,
        handle: &GameCardHandle,
        partition: GameCardPartition,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenGameCardFileSystemIn {
            handle: *handle,
            partition: partition as u32,
        };
        let raw =
            cmif::proxy::open_game_card_file_system(guard.domain(), self.inner.ctx(), &input)?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn extend_save_data_file_system(
        &self,
        space_id: SaveDataSpaceId,
        save_id: u64,
        data_size: i64,
        journal_size: i64,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = ExtendSaveDataIn {
            save_data_space_id: space_id as u8,
            pad: [0; 7],
            save_id,
            data_size,
            journal_size,
        };
        cmif::proxy::extend_save_data_file_system(guard.domain(), self.inner.ctx(), &input)
    }

    pub fn open_save_data_file_system(
        &self,
        space_id: SaveDataSpaceId,
        attr: &SaveDataAttribute,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenSaveDataIn {
            save_data_space_id: space_id as u8,
            pad: [0; 7],
            attr: *attr,
        };
        let raw =
            cmif::proxy::open_save_data_file_system(guard.domain(), self.inner.ctx(), &input)?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_save_data_file_system_by_system_save_data_id(
        &self,
        space_id: SaveDataSpaceId,
        attr: &SaveDataAttribute,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenSaveDataIn {
            save_data_space_id: space_id as u8,
            pad: [0; 7],
            attr: *attr,
        };
        let raw = cmif::proxy::open_save_data_file_system_by_system_save_data_id(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_read_only_save_data_file_system(
        &self,
        space_id: SaveDataSpaceId,
        attr: &SaveDataAttribute,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenSaveDataIn {
            save_data_space_id: space_id as u8,
            pad: [0; 7],
            attr: *attr,
        };
        let raw = cmif::proxy::open_read_only_save_data_file_system(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn read_save_data_file_system_extra_data_by_save_data_space_id(
        &self,
        buf: &mut [u8],
        space_id: SaveDataSpaceId,
        save_id: u64,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = ReadExtraDataBySpaceIdIn {
            save_data_space_id: space_id as u8,
            _pad: [0; 7],
            save_id,
        };
        cmif::proxy::read_save_data_file_system_extra_data_by_save_data_space_id(
            guard.domain(),
            self.inner.ctx(),
            &input,
            buf,
        )
    }

    pub fn read_save_data_file_system_extra_data(
        &self,
        buf: &mut [u8],
        save_id: u64,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::read_save_data_file_system_extra_data(
            guard.domain(),
            self.inner.ctx(),
            save_id,
            buf,
        )
    }

    pub fn write_save_data_file_system_extra_data(
        &self,
        buf: &[u8],
        space_id: SaveDataSpaceId,
        save_id: u64,
    ) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = WriteExtraDataIn {
            save_data_space_id: space_id as u8,
            _pad: [0; 7],
            save_id,
        };
        cmif::proxy::write_save_data_file_system_extra_data(
            guard.domain(),
            self.inner.ctx(),
            &input,
            buf,
        )
    }

    pub fn open_save_data_info_reader(
        &self,
        space_id: SaveDataSpaceId,
    ) -> Result<FsSaveDataInfoReader<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = if space_id == SaveDataSpaceId::All {
            cmif::proxy::open_save_data_info_reader(guard.domain(), self.inner.ctx())?
        } else {
            cmif::proxy::open_save_data_info_reader_by_save_data_space_id(
                guard.domain(),
                self.inner.ctx(),
                space_id as u8,
            )?
        };
        Ok(FsSaveDataInfoReader {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_save_data_info_reader_with_filter(
        &self,
        space_id: SaveDataSpaceId,
        filter: &SaveDataFilter,
    ) -> Result<FsSaveDataInfoReader<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenSaveDataInfoReaderWithFilterIn {
            save_data_space_id: space_id as u8,
            pad: [0; 7],
            filter: *filter,
        };
        let raw = cmif::proxy::open_save_data_info_reader_with_filter(
            guard.domain(),
            self.inner.ctx(),
            &input,
        )?;
        Ok(FsSaveDataInfoReader {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_image_directory_file_system(
        &self,
        id: ImageDirectoryId,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_image_directory_file_system(
            guard.domain(),
            self.inner.ctx(),
            id as u32,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_content_storage_file_system(
        &self,
        id: ContentStorageId,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_content_storage_file_system(
            guard.domain(),
            self.inner.ctx(),
            id as u32,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_custom_storage_file_system(
        &self,
        id: CustomStorageId,
    ) -> Result<FsFileSystem<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_custom_storage_file_system(
            guard.domain(),
            self.inner.ctx(),
            id as u32,
        )?;
        Ok(FsFileSystem {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_data_storage_by_current_process(&self) -> Result<FsStorage<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw =
            cmif::proxy::open_data_storage_by_current_process(guard.domain(), self.inner.ctx())?;
        Ok(FsStorage {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_data_storage_by_program_id(
        &self,
        program_id: u64,
    ) -> Result<FsStorage<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_data_storage_by_program_id(
            guard.domain(),
            self.inner.ctx(),
            program_id,
        )?;
        Ok(FsStorage {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_data_storage_by_data_id(
        &self,
        data_id: u64,
        storage_id: NcmStorageId,
    ) -> Result<FsStorage<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let input = OpenDataStorageByDataIdIn {
            storage_id: storage_id as u8,
            _pad: [0; 7],
            data_id,
        };
        let raw =
            cmif::proxy::open_data_storage_by_data_id(guard.domain(), self.inner.ctx(), &input)?;
        Ok(FsStorage {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_patch_data_storage_by_current_process(
        &self,
    ) -> Result<FsStorage<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_patch_data_storage_by_current_process(
            guard.domain(),
            self.inner.ctx(),
        )?;
        Ok(FsStorage {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_device_operator(&self) -> Result<FsDeviceOperator<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw = cmif::proxy::open_device_operator(guard.domain(), self.inner.ctx())?;
        Ok(FsDeviceOperator {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn open_sd_card_detection_event_notifier(
        &self,
    ) -> Result<FsEventNotifier<'_>, DispatchError> {
        let guard = self.inner.pool.acquire();
        let raw =
            cmif::proxy::open_sd_card_detection_event_notifier(guard.domain(), self.inner.ctx())?;
        Ok(FsEventNotifier {
            object_id: raw,
            ctx: &self.inner,
        })
    }

    pub fn is_signed_system_partition_on_sd_card_valid(&self) -> Result<bool, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::is_signed_system_partition_on_sd_card_valid(guard.domain(), self.inner.ctx())
    }

    pub fn get_program_id(
        &self,
        path: &[u8; FS_MAX_PATH],
        attr: ContentAttributes,
    ) -> Result<u64, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_program_id(guard.domain(), self.inner.ctx(), path, attr as u8)
    }

    pub fn get_rights_id_by_path(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<RightsId, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_rights_id_by_path(guard.domain(), self.inner.ctx(), path)
    }

    pub fn get_rights_id_and_key_generation_by_path(
        &self,
        path: &[u8; FS_MAX_PATH],
        attr: ContentAttributes,
    ) -> Result<(u8, RightsId), DispatchError> {
        let guard = self.inner.pool.acquire();
        let out = cmif::proxy::get_rights_id_and_key_generation_by_path(
            guard.domain(),
            self.inner.ctx(),
            path,
            true,
            attr as u8,
        )?;
        Ok((out.key_generation, out.rights_id))
    }

    pub fn get_rights_id_and_key_generation_by_path_legacy(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<(u8, RightsId), DispatchError> {
        let guard = self.inner.pool.acquire();
        let out = cmif::proxy::get_rights_id_and_key_generation_by_path(
            guard.domain(),
            self.inner.ctx(),
            path,
            false,
            0,
        )?;
        Ok((out.key_generation, out.rights_id))
    }

    pub fn get_and_clear_error_info(&self) -> Result<FileSystemProxyErrorInfo, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_and_clear_error_info(guard.domain(), self.inner.ctx())
    }

    pub fn get_content_storage_info_index(&self) -> Result<i32, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_content_storage_info_index(guard.domain(), self.inner.ctx())
    }

    pub fn disable_auto_save_data_creation(&self) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::disable_auto_save_data_creation(guard.domain(), self.inner.ctx())
    }

    pub fn set_global_access_log_mode(&self, mode: u32) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::set_global_access_log_mode(guard.domain(), self.inner.ctx(), mode)
    }

    pub fn get_global_access_log_mode(&self) -> Result<u32, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_global_access_log_mode(guard.domain(), self.inner.ctx())
    }

    pub fn output_access_log_to_sd_card(&self, log: &[u8]) -> Result<(), DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::output_access_log_to_sd_card(guard.domain(), self.inner.ctx(), log)
    }

    pub fn get_program_index_for_access_log(&self) -> Result<(u32, u32), DispatchError> {
        let guard = self.inner.pool.acquire();
        let out = cmif::proxy::get_program_index_for_access_log(guard.domain(), self.inner.ctx())?;
        Ok((out.index, out.count))
    }

    pub fn get_and_clear_memory_report_info(&self) -> Result<MemoryReportInfo, DispatchError> {
        let guard = self.inner.pool.acquire();
        cmif::proxy::get_and_clear_memory_report_info(guard.domain(), self.inner.ctx())
    }
}

// ---------------------------------------------------------------------------
// Sub-object helper macro
// ---------------------------------------------------------------------------

macro_rules! sub_object_drop {
    ($ty:ident) => {
        impl Drop for $ty<'_> {
            fn drop(&mut self) {
                let guard = self.ctx.pool.acquire();
                let _ = unsafe { guard.open_for_close(self.object_id) };
            }
        }
    };
}

// ---------------------------------------------------------------------------
// FsFileSystem (IFileSystem sub-object)
// ---------------------------------------------------------------------------

pub struct FsFileSystem<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsFileSystem);

impl<'svc> FsFileSystem<'svc> {
    pub fn create_file(
        &self,
        path: &[u8; FS_MAX_PATH],
        size: i64,
        option: CreateOption,
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::create_file(&object, self.ctx.ctx(), path, size, option.bits())
    }

    pub fn delete_file(&self, path: &[u8; FS_MAX_PATH]) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_path(&object, self.ctx.ctx(), proto::FS_DELETE_FILE, path)
    }

    pub fn create_directory(&self, path: &[u8; FS_MAX_PATH]) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_path(&object, self.ctx.ctx(), proto::FS_CREATE_DIRECTORY, path)
    }

    pub fn delete_directory(&self, path: &[u8; FS_MAX_PATH]) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_path(&object, self.ctx.ctx(), proto::FS_DELETE_DIRECTORY, path)
    }

    pub fn delete_directory_recursively(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_path(
            &object,
            self.ctx.ctx(),
            proto::FS_DELETE_DIRECTORY_RECURSIVELY,
            path,
        )
    }

    pub fn rename_file(
        &self,
        cur_path: &[u8; FS_MAX_PATH],
        new_path: &[u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_two_paths(
            &object,
            self.ctx.ctx(),
            proto::FS_RENAME_FILE,
            cur_path,
            new_path,
        )
    }

    pub fn rename_directory(
        &self,
        cur_path: &[u8; FS_MAX_PATH],
        new_path: &[u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_two_paths(
            &object,
            self.ctx.ctx(),
            proto::FS_RENAME_DIRECTORY,
            cur_path,
            new_path,
        )
    }

    pub fn get_entry_type(&self, path: &[u8; FS_MAX_PATH]) -> Result<DirEntryType, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        let raw = cmif::filesystem::get_entry_type(&object, self.ctx.ctx(), path)?;
        Ok(match raw {
            0 => DirEntryType::Dir,
            _ => DirEntryType::File,
        })
    }

    pub fn open_file(
        &self,
        path: &[u8; FS_MAX_PATH],
        mode: OpenMode,
    ) -> Result<FsFile<'svc>, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        let raw = cmif::filesystem::open_file(&object, self.ctx.ctx(), path, mode.bits())?;
        Ok(FsFile {
            object_id: raw,
            ctx: self.ctx,
        })
    }

    pub fn open_directory(
        &self,
        path: &[u8; FS_MAX_PATH],
        mode: DirOpenMode,
    ) -> Result<FsDir<'svc>, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        let raw = cmif::filesystem::open_directory(&object, self.ctx.ctx(), path, mode.bits())?;
        Ok(FsDir {
            object_id: raw,
            ctx: self.ctx,
        })
    }

    pub fn commit(&self) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::commit(&object, self.ctx.ctx())
    }

    pub fn get_free_space(&self, path: &[u8; FS_MAX_PATH]) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::get_space(&object, self.ctx.ctx(), proto::FS_GET_FREE_SPACE, path)
    }

    pub fn get_total_space(&self, path: &[u8; FS_MAX_PATH]) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::get_space(&object, self.ctx.ctx(), proto::FS_GET_TOTAL_SPACE, path)
    }

    pub fn clean_directory_recursively(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::cmd_with_path(
            &object,
            self.ctx.ctx(),
            proto::FS_CLEAN_DIRECTORY_RECURSIVELY,
            path,
        )
    }

    pub fn get_file_time_stamp_raw(
        &self,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<TimeStampRaw, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::get_file_time_stamp_raw(&object, self.ctx.ctx(), path)
    }

    pub fn query_entry(
        &self,
        path: &[u8; FS_MAX_PATH],
        query_id: FileSystemQueryId,
        in_buf: &[u8],
        out_buf: &mut [u8],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::query_entry(
            &object,
            self.ctx.ctx(),
            path,
            query_id as u32,
            in_buf,
            out_buf,
        )
    }

    pub fn get_file_system_attribute(&self) -> Result<FileSystemAttribute, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("filesystem object_id obtained from server");
        cmif::filesystem::get_file_system_attribute(&object, self.ctx.ctx())
    }
}

// ---------------------------------------------------------------------------
// FsFile (IFile sub-object)
// ---------------------------------------------------------------------------

pub struct FsFile<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsFile);

impl FsFile<'_> {
    pub fn read(
        &self,
        offset: i64,
        buf: &mut [u8],
        read_size: u64,
        option: ReadOption,
    ) -> Result<u64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::read(
            &object,
            self.ctx.ctx(),
            offset,
            buf,
            read_size,
            option.bits(),
        )
    }

    pub fn write(
        &self,
        offset: i64,
        buf: &[u8],
        write_size: u64,
        option: WriteOption,
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::write(
            &object,
            self.ctx.ctx(),
            offset,
            buf,
            write_size,
            option.bits(),
        )
    }

    pub fn flush(&self) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::flush(&object, self.ctx.ctx())
    }

    pub fn set_size(&self, size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::set_size(&object, self.ctx.ctx(), size)
    }

    pub fn get_size(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::get_size(&object, self.ctx.ctx())
    }

    pub fn operate_range(
        &self,
        op_id: OperationId,
        offset: i64,
        len: i64,
    ) -> Result<RangeInfo, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("file object_id obtained from server");
        cmif::file::operate_range(&object, self.ctx.ctx(), op_id as u32, offset, len)
    }
}

// ---------------------------------------------------------------------------
// FsDir (IDirectory sub-object)
// ---------------------------------------------------------------------------

pub struct FsDir<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsDir);

impl FsDir<'_> {
    pub fn read(&self, buf: &mut [DirectoryEntry]) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("directory object_id obtained from server");
        cmif::dir::read(&object, self.ctx.ctx(), buf)
    }

    pub fn get_entry_count(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("directory object_id obtained from server");
        cmif::dir::get_entry_count(&object, self.ctx.ctx())
    }
}

// ---------------------------------------------------------------------------
// FsStorage (IStorage sub-object)
// ---------------------------------------------------------------------------

pub struct FsStorage<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsStorage);

impl FsStorage<'_> {
    pub fn read(&self, offset: i64, buf: &mut [u8], read_size: u64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::read(&object, self.ctx.ctx(), offset, buf, read_size)
    }

    pub fn write(&self, offset: i64, buf: &[u8], write_size: u64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::write(&object, self.ctx.ctx(), offset, buf, write_size)
    }

    pub fn flush(&self) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::flush(&object, self.ctx.ctx())
    }

    pub fn set_size(&self, size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::set_size(&object, self.ctx.ctx(), size)
    }

    pub fn get_size(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::get_size(&object, self.ctx.ctx())
    }

    pub fn operate_range(
        &self,
        op_id: OperationId,
        offset: i64,
        len: i64,
    ) -> Result<RangeInfo, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("storage object_id obtained from server");
        cmif::storage::operate_range(&object, self.ctx.ctx(), op_id as u32, offset, len)
    }
}

// ---------------------------------------------------------------------------
// FsSaveDataInfoReader (ISaveDataInfoReader sub-object)
// ---------------------------------------------------------------------------

pub struct FsSaveDataInfoReader<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsSaveDataInfoReader);

impl FsSaveDataInfoReader<'_> {
    pub fn read(&self, buf: &mut [SaveDataInfo]) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("save data info reader object_id obtained from server");
        cmif::save_data_info_reader::read(&object, self.ctx.ctx(), buf)
    }
}

// ---------------------------------------------------------------------------
// FsEventNotifier (IEventNotifier sub-object)
// ---------------------------------------------------------------------------

pub struct FsEventNotifier<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsEventNotifier);

impl FsEventNotifier<'_> {
    pub fn get_event_handle(&self) -> Result<u32, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("event notifier object_id obtained from server");
        cmif::event_notifier::get_event_handle(&object, self.ctx.ctx())
    }
}

// ---------------------------------------------------------------------------
// FsDeviceOperator (IDeviceOperator sub-object)
// ---------------------------------------------------------------------------

pub struct FsDeviceOperator<'svc> {
    object_id: u32,
    ctx: &'svc FsContext,
}

sub_object_drop!(FsDeviceOperator);

impl FsDeviceOperator<'_> {
    pub fn is_sd_card_inserted(&self) -> Result<bool, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::is_sd_card_inserted(&object, self.ctx.ctx())
    }

    pub fn get_sd_card_speed_mode(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_sd_card_speed_mode(&object, self.ctx.ctx())
    }

    pub fn get_sd_card_cid(&self, dst: &mut [u8], size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_sd_card_cid(&object, self.ctx.ctx(), dst, size)
    }

    pub fn get_sd_card_user_area_size(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_sd_card_user_area_size(&object, self.ctx.ctx())
    }

    pub fn get_sd_card_protected_area_size(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_sd_card_protected_area_size(&object, self.ctx.ctx())
    }

    pub fn get_and_clear_sd_card_error_info(
        &self,
        size: i64,
        dst: &mut [u8],
    ) -> Result<(StorageErrorInfo, i64), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        let out = cmif::device_operator::get_and_clear_storage_error_info(
            &object,
            self.ctx.ctx(),
            proto::DEVICE_OPERATOR_GET_AND_CLEAR_SD_CARD_ERROR_INFO,
            size,
            dst,
        )?;
        Ok((out.error_info, out.log_size))
    }

    pub fn get_mmc_cid(&self, dst: &mut [u8], size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_mmc_cid(&object, self.ctx.ctx(), dst, size)
    }

    pub fn get_mmc_speed_mode(&self) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_mmc_speed_mode(&object, self.ctx.ctx())
    }

    pub fn get_mmc_patrol_count(&self) -> Result<u32, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_mmc_patrol_count(&object, self.ctx.ctx())
    }

    pub fn get_and_clear_mmc_error_info(
        &self,
        size: i64,
        dst: &mut [u8],
    ) -> Result<(StorageErrorInfo, i64), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        let out = cmif::device_operator::get_and_clear_storage_error_info(
            &object,
            self.ctx.ctx(),
            proto::DEVICE_OPERATOR_GET_AND_CLEAR_MMC_ERROR_INFO,
            size,
            dst,
        )?;
        Ok((out.error_info, out.log_size))
    }

    pub fn get_mmc_extended_csd(&self, dst: &mut [u8], size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_mmc_extended_csd(&object, self.ctx.ctx(), dst, size)
    }

    pub fn is_game_card_inserted(&self) -> Result<bool, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::is_game_card_inserted(&object, self.ctx.ctx())
    }

    pub fn get_game_card_handle(&self) -> Result<GameCardHandle, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_handle(&object, self.ctx.ctx())
    }

    pub fn get_game_card_update_partition_info(
        &self,
        handle: &GameCardHandle,
    ) -> Result<GameCardUpdatePartitionInfo, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_update_partition_info(&object, self.ctx.ctx(), handle)
    }

    pub fn get_game_card_attribute(&self, handle: &GameCardHandle) -> Result<u8, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_attribute(&object, self.ctx.ctx(), handle)
    }

    pub fn get_game_card_device_certificate_legacy(
        &self,
        handle: &GameCardHandle,
        size: i64,
        dst: &mut [u8],
    ) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_device_certificate_legacy(
            &object,
            self.ctx.ctx(),
            handle,
            size,
            dst,
        )
    }

    pub fn get_game_card_device_certificate(
        &self,
        handle: &GameCardHandle,
        size: i64,
        dst: &mut [u8],
    ) -> Result<i64, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_device_certificate(
            &object,
            self.ctx.ctx(),
            handle,
            size,
            dst,
        )
    }

    pub fn get_game_card_id_set(&self, dst: &mut [u8], size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_id_set(&object, self.ctx.ctx(), dst, size)
    }

    pub fn get_game_card_error_report_info(
        &self,
    ) -> Result<GameCardErrorReportInfo, DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_error_report_info(&object, self.ctx.ctx())
    }

    pub fn get_game_card_device_id(&self, dst: &mut [u8], size: i64) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::get_game_card_device_id(&object, self.ctx.ctx(), dst, size)
    }

    pub fn challenge_card_existence(
        &self,
        handle: &GameCardHandle,
        dst: &mut [u8],
        seed: &[u8],
        value: &[u8],
    ) -> Result<(), DispatchError> {
        let guard = self.ctx.pool.acquire();
        let object = unsafe { guard.open_transient(self.object_id) }
            .expect("device operator object_id obtained from server");
        cmif::device_operator::challenge_card_existence(
            &object,
            self.ctx.ctx(),
            handle,
            dst,
            seed,
            value,
        )
    }
}

// ---------------------------------------------------------------------------
// connect_cmif
// ---------------------------------------------------------------------------

pub fn connect_cmif(sm: &SmService) -> Result<FsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::new(handle);
    let pointer_buffer_size = session.pointer_buffer_size();

    let creator = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    cmif::proxy::set_current_process(&creator, 0).map_err(ConnectCmifError::SetCurrentProcess)?;

    let pool_size = proto::FS_POOL_SIZE;
    let mut sessions: Vec<Domain> = Vec::with_capacity(pool_size);
    sessions.push(creator);
    for _ in 1..pool_size {
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        let cloned_domain =
            unsafe { Domain::from_handle_unchecked(cloned_handle, pointer_buffer_size) };
        sessions.push(cloned_domain);
    }

    let pool = SessionPool::new(sessions.into_boxed_slice());

    Ok(FsService {
        inner: FsContext {
            pool,
            priority: AtomicU32::new(Priority::Normal as u32),
        },
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    #[error("failed to look up fsp-srv service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    #[error("failed to ConvertToDomain on fsp-srv session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    #[error("failed to SetCurrentProcess on fsp-srv")]
    SetCurrentProcess(#[source] DispatchError),
    #[error("failed to clone fsp-srv session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
}
