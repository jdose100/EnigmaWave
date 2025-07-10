use super::Locked;
use alloc::alloc::Layout;
use core::{alloc::GlobalAlloc, mem, ptr, ptr::NonNull};

/// Размеры блоков для использования.
///
/// Каждый из размеров должен быть в степени 2, поскольку они также
/// используются в качестве выравнивания блока (выравнивания всегда
/// должны быть в степени 2).
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct ListNode {
    next: Option<&'static mut ListNode>,
}

#[derive(Default)]
pub struct FixedSizeBlockAllocator {
    list_heads: [
        Option<&'static mut ListNode>; BLOCK_SIZES.len()
    ] = [const { None }; BLOCK_SIZES.len()],

    fallback_allocator: linked_list_allocator::Heap = linked_list_allocator::Heap::empty(),
}

impl FixedSizeBlockAllocator {
    /// Создает пустой FixedSizeBlockAllocator.
    pub const fn new() -> Self {
        Self {
            list_heads: [const { None }; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Инициализируйте распределитель с заданными границами кучи.
    ///
    /// ## Safety
    ///
    /// Эта функция небезопасна, поскольку вызывающий метод должен
    /// гарантировать, что заданные границы кучи допустимы и что куча не
    /// используется. Этот метод должен вызываться только один раз.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator.init(heap_start, heap_size);
        }
    }

    /// Распределяет с помощью резервного распределителя.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // В списке нет блока => выделить новый блок.
                        let block_size = BLOCK_SIZES[index];

                        // Работает только в том случае, если все размеры
                        // блоков равны степени 2.
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        allocator.fallback_alloc(layout)
                    }
                }
            }
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                // Убедитесь, что блок имеет размер и выравнивание,
                // необходимые для хранения узла.
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}

/// Выберите подходящий размер блока для данного макета.
///
/// Возвращает индекс в массив `BLOCK_SIZES`.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}
