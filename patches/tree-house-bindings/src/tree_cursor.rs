use ::std::os::raw;
use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::{c_char, CStr};
use std::marker::PhantomData;
use std::{fmt, mem};

use crate::node::NodeRaw;
use crate::{Node, Tree};

thread_local! {
    static CACHE: Cell<Option<TreeCursorGuard>> = const { Cell::new(None) };
}

#[repr(C)]
#[derive(Clone)]
struct TreeCursorRaw {
    tree: *const raw::c_void,
    id: *const raw::c_void,
    context: [u32; 3usize],
}

#[repr(C)]
struct TreeCursorGuard(TreeCursorRaw);

impl Drop for TreeCursorGuard {
    fn drop(&mut self) {
        unsafe { ts_tree_cursor_delete(&mut self.0) }
    }
}

pub struct TreeCursor<'a> {
    inner: TreeCursorRaw,
    tree: PhantomData<&'a Tree>,
}

impl<'tree> TreeCursor<'tree> {
    pub(crate) fn new(node: &Node<'tree>) -> Self {
        Self {
            inner: match CACHE.take() {
                Some(guard) => unsafe {
                    let mut cursor = guard.0.clone();
                    mem::forget(guard);
                    ts_tree_cursor_reset(&mut cursor, node.as_raw());
                    cursor
                },
                None => unsafe { ts_tree_cursor_new(node.as_raw()) },
            },
            tree: PhantomData,
        }
    }

    pub fn goto_parent(&mut self) -> bool {
        unsafe { ts_tree_cursor_goto_parent(&mut self.inner) }
    }

    pub fn goto_next_sibling(&mut self) -> bool {
        unsafe { ts_tree_cursor_goto_next_sibling(&mut self.inner) }
    }

    pub fn goto_previous_sibling(&mut self) -> bool {
        unsafe { ts_tree_cursor_goto_previous_sibling(&mut self.inner) }
    }

    pub fn goto_first_child(&mut self) -> bool {
        unsafe { ts_tree_cursor_goto_first_child(&mut self.inner) }
    }

    pub fn goto_last_child(&mut self) -> bool {
        unsafe { ts_tree_cursor_goto_last_child(&mut self.inner) }
    }

    pub fn goto_first_child_for_byte(&mut self, byte_idx: u32) -> Option<u32> {
        match unsafe { ts_tree_cursor_goto_first_child_for_byte(&mut self.inner, byte_idx) } {
            -1 => None,
            n => Some(n as u32),
        }
    }

    pub fn reset(&mut self, node: &Node<'tree>) {
        unsafe { ts_tree_cursor_reset(&mut self.inner, node.as_raw()) }
    }

    pub fn node(&self) -> Node<'tree> {
        unsafe { Node::from_raw(ts_tree_cursor_current_node(&self.inner)).unwrap_unchecked() }
    }

    pub fn field_name(&self) -> Option<&'tree str> {
        unsafe {
            let ptr = ts_tree_cursor_current_field_name(&self.inner);
            (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_str().unwrap())
        }
    }
}

impl fmt::Debug for TreeCursorRaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InactiveTreeCursor").finish_non_exhaustive()
    }
}

impl Drop for TreeCursor<'_> {
    fn drop(&mut self) {
        CACHE.set(Some(TreeCursorGuard(self.inner.clone())))
    }
}

impl Clone for TreeCursor<'_> {
    fn clone(&self) -> Self {
        TreeCursor {
            inner: unsafe { ts_tree_cursor_copy(&self.inner) },
            tree: PhantomData,
        }
    }
}

impl<'cursor, 'tree: 'cursor> IntoIterator for &'cursor mut TreeCursor<'tree> {
    type Item = Node<'tree>;
    type IntoIter = TreeRecursiveWalker<'cursor, 'tree>;

    fn into_iter(self) -> Self::IntoIter {
        let mut queue = VecDeque::new();
        let root = self.node();
        queue.push_back(root.clone());

        TreeRecursiveWalker {
            cursor: self,
            queue,
            root,
        }
    }
}

pub struct TreeRecursiveWalker<'cursor, 'tree: 'cursor> {
    cursor: &'cursor mut TreeCursor<'tree>,
    queue: VecDeque<Node<'tree>>,
    root: Node<'tree>,
}

impl<'tree> Iterator for TreeRecursiveWalker<'_, 'tree> {
    type Item = Node<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cursor.node();

        if current != self.root && self.cursor.goto_next_sibling() {
            self.queue.push_back(current);
            return Some(self.cursor.node());
        }

        while let Some(queued) = self.queue.pop_front() {
            self.cursor.reset(&queued);

            if !self.cursor.goto_first_child() {
                continue;
            }

            return Some(self.cursor.node());
        }

        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
extern "C" {
    fn ts_tree_cursor_new(node: NodeRaw) -> TreeCursorRaw;
    fn ts_tree_cursor_delete(self_: *mut TreeCursorRaw);
    fn ts_tree_cursor_reset(self_: *mut TreeCursorRaw, node: NodeRaw);
    fn ts_tree_cursor_current_node(self_: *const TreeCursorRaw) -> NodeRaw;
    fn ts_tree_cursor_goto_parent(self_: *mut TreeCursorRaw) -> bool;
    fn ts_tree_cursor_goto_next_sibling(self_: *mut TreeCursorRaw) -> bool;
    fn ts_tree_cursor_goto_previous_sibling(self_: *mut TreeCursorRaw) -> bool;
    fn ts_tree_cursor_goto_first_child(self_: *mut TreeCursorRaw) -> bool;
    fn ts_tree_cursor_goto_last_child(self_: *mut TreeCursorRaw) -> bool;
    fn ts_tree_cursor_goto_first_child_for_byte(self_: *mut TreeCursorRaw, goal_byte: u32) -> i64;
    fn ts_tree_cursor_copy(cursor: *const TreeCursorRaw) -> TreeCursorRaw;
    fn ts_tree_cursor_current_field_name(cursor: *const TreeCursorRaw) -> *const c_char;
}

#[cfg(target_arch = "wasm32")]
mod wasm_cursor_ffi {
    use super::*;
    use std::ffi::c_char;

    extern "C" {
        // ABI-adjusted: sret for TreeCursorRaw return, pointer for NodeRaw arg
        #[link_name = "ts_tree_cursor_new"]
        fn __ts_tree_cursor_new(result: *mut TreeCursorRaw, node: *const NodeRaw);

        // ABI-adjusted: pointer for NodeRaw arg
        #[link_name = "ts_tree_cursor_reset"]
        fn __ts_tree_cursor_reset(self_: *mut TreeCursorRaw, node: *const NodeRaw);

        // ABI-adjusted: sret for NodeRaw return
        #[link_name = "ts_tree_cursor_current_node"]
        fn __ts_tree_cursor_current_node(result: *mut NodeRaw, self_: *const TreeCursorRaw);

        // ABI-adjusted: sret for TreeCursorRaw return
        #[link_name = "ts_tree_cursor_copy"]
        fn __ts_tree_cursor_copy(result: *mut TreeCursorRaw, cursor: *const TreeCursorRaw);

        // These functions only use pointers/primitives — no ABI mismatch
        pub(super) fn ts_tree_cursor_delete(self_: *mut TreeCursorRaw);
        pub(super) fn ts_tree_cursor_goto_parent(self_: *mut TreeCursorRaw) -> bool;
        pub(super) fn ts_tree_cursor_goto_next_sibling(self_: *mut TreeCursorRaw) -> bool;
        pub(super) fn ts_tree_cursor_goto_previous_sibling(self_: *mut TreeCursorRaw) -> bool;
        pub(super) fn ts_tree_cursor_goto_first_child(self_: *mut TreeCursorRaw) -> bool;
        pub(super) fn ts_tree_cursor_goto_last_child(self_: *mut TreeCursorRaw) -> bool;
        pub(super) fn ts_tree_cursor_goto_first_child_for_byte(
            self_: *mut TreeCursorRaw,
            goal_byte: u32,
        ) -> i64;
        pub(super) fn ts_tree_cursor_current_field_name(
            cursor: *const TreeCursorRaw,
        ) -> *const c_char;
    }

    pub(super) unsafe fn ts_tree_cursor_new(node: NodeRaw) -> TreeCursorRaw {
        let mut result = std::mem::MaybeUninit::uninit();
        __ts_tree_cursor_new(result.as_mut_ptr(), &node);
        result.assume_init()
    }

    pub(super) unsafe fn ts_tree_cursor_reset(self_: *mut TreeCursorRaw, node: NodeRaw) {
        __ts_tree_cursor_reset(self_, &node)
    }

    pub(super) unsafe fn ts_tree_cursor_current_node(self_: *const TreeCursorRaw) -> NodeRaw {
        let mut result = std::mem::MaybeUninit::uninit();
        __ts_tree_cursor_current_node(result.as_mut_ptr(), self_);
        result.assume_init()
    }

    pub(super) unsafe fn ts_tree_cursor_copy(cursor: *const TreeCursorRaw) -> TreeCursorRaw {
        let mut result = std::mem::MaybeUninit::uninit();
        __ts_tree_cursor_copy(result.as_mut_ptr(), cursor);
        result.assume_init()
    }
}
#[cfg(target_arch = "wasm32")]
use wasm_cursor_ffi::*;
