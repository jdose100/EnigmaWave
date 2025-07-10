//! В данном модуле находится реализация работы с памятью, а именно работа
//! со страницами и их инициализация (Paging).

use bootloader_api::info::{MemoryRegionKind::Usable, MemoryRegions};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

/// Инициализирует новую таблицу OffsetPageTable.
///
/// ## Safety
///
/// Эта функция небезопасна, поскольку вызывающий объект должен
/// гарантировать, что вся физическая память будет преобразована
/// в виртуальную при переданном `physical_memory_offset`.
/// Кроме того, эта функция должна вызываться только один раз, чтобы
/// избежать наложения псевдонимов на ссылки "&mut"
/// (что является неопределенным поведением).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// FrameworAllocator, который возвращает используемые
/// кадры из карты памяти загрузчика.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Создаёт Frameworkallocator из переданной карты памяти.
    ///
    /// ## Safety
    ///
    /// Эта функция небезопасна, поскольку вызывающий объект должен гарантировать,
    /// что переданная карта памяти действительна. Основное требование заключается
    /// в том, что все кадры, помеченные в ней как "ПРИГОДНЫЕ для использования",
    /// на самом деле не используются.
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        Self {
            memory_map,
            next: 0,
        }
    }

    /// Возвращает итератор по используемым кадрам, указанным в карте памяти.
    pub fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();

        let usable_regions = regions.filter(|region| region.kind == Usable);
        let address_ranges = usable_regions.map(|region| region.start..region.end);
        let frame_addresses = address_ranges.flat_map(|region| region.step_by(4096));

        frame_addresses.map(|address| PhysFrame::containing_address(PhysAddr::new(address)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Возвращает изменяемую ссылку на таблицу активного уровня 4.
///
/// ## Safety
///
/// Эта функция небезопасна, поскольку вызывающий объект должен гарантировать,
/// что вся физическая память будет преобразована в виртуальную при переданном
/// `physical_memory_offset`. Кроме того, эта функция должна вызываться только
/// один раз, чтобы избежать наложения псевдонимов на ссылки "&mut"
/// (что является неопределенным поведением).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}
