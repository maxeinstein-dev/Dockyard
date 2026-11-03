//! The dock for assistive technology (spec 0011 H3): a UI Automation
//! provider, so the Narrator and the other screen readers read the dock as a
//! toolbar of buttons, each with its name, and press them.
//!
//! What the provider says comes from `ENTRIES`, which the dock keeps up to
//! date; UI Automation may call from its own threads. A press goes back to the
//! dock through `post`.
//!
//! The dock takes the keyboard by its hotkey (M14): the item chosen has the
//! focus (`set_focus`), and UI Automation hears of it, of items that come
//! and go and of names that change.
//!
//! Another window of the dock's (the preview card, spec 0015) uses it too,
//! with `publish_for`: its own name, and its own way to press, which must not
//! block (UI Automation's thread).

// The patterns below match Windows' constants, which keep their own names.
#![allow(non_upper_case_globals)]

use std::sync::{Mutex, PoisonError};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{SafeArrayCreateVector, SafeArrayPutElement};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    IInvokeProvider, IInvokeProvider_Impl, IRawElementProviderFragment,
    IRawElementProviderFragment_Impl, IRawElementProviderFragmentRoot,
    IRawElementProviderFragmentRoot_Impl, IRawElementProviderSimple,
    IRawElementProviderSimple_Impl, NavigateDirection, NavigateDirection_FirstChild,
    NavigateDirection_LastChild, NavigateDirection_NextSibling, NavigateDirection_Parent,
    NavigateDirection_PreviousSibling, ProviderOptions, ProviderOptions_ServerSideProvider,
    StructureChangeType_ChildrenInvalidated, UIA_AutomationFocusChangedEventId,
    UIA_AutomationIdPropertyId, UIA_ButtonControlTypeId, UIA_ControlTypePropertyId,
    UIA_HasKeyboardFocusPropertyId, UIA_InvokePatternId, UIA_IsEnabledPropertyId,
    UIA_IsKeyboardFocusablePropertyId, UIA_NamePropertyId, UIA_PATTERN_ID, UIA_PROPERTY_ID,
    UIA_ToolBarControlTypeId, UiaAppendRuntimeId, UiaClientsAreListening, UiaHostProviderFromHwnd,
    UiaRaiseAutomationEvent, UiaRaiseAutomationPropertyChangedEvent, UiaRaiseStructureChangedEvent,
    UiaRect, UiaReturnRawElementProvider, UiaRootObjectId,
};
use windows::core::{BSTR, Error, IUnknown, IUnknownImpl, Interface, Result, implement};

/// One item as assistive technology sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The item's key: stable across changes of the list.
    pub key: String,
    pub name: String,
    /// Its box on the screen, in physical pixels.
    pub rect: (f64, f64, f64, f64),
}

/// How a window other than a dock presses one of its items: its window and
/// the item's key.
pub type Press = fn(isize, &str);

/// What a window shows to assistive technology.
struct Published {
    hwnd: isize,
    /// Its name; the dock's is `LABEL`.
    label: String,
    entries: Vec<Entry>,
    /// `None` for a dock: the press goes to it through `post_to`.
    press: Option<Press>,
}

/// Each window's items: one dock per monitor, and the preview card.
static ENTRIES: Mutex<Vec<Published>> = Mutex::new(Vec::new());

/// The item chosen by the keyboard on each dock that has the keyboard: its
/// window and the item's key.
static FOCUS: Mutex<Vec<(isize, String)>> = Mutex::new(Vec::new());

/// What the dock of the window `hwnd` shows now.
pub fn publish(hwnd: isize, entries: Vec<Entry>) {
    put(Published {
        hwnd,
        label: LABEL.to_owned(),
        entries,
        press: None,
    });
}

/// What the window `hwnd`, named `label`, shows now; `press` presses its
/// items.
pub fn publish_for(hwnd: isize, label: String, entries: Vec<Entry>, press: Press) {
    put(Published {
        hwnd,
        label,
        entries,
        press: Some(press),
    });
}

fn put(published: Published) {
    let hwnd = published.hwnd;
    let changed = {
        let mut all = ENTRIES.lock().unwrap_or_else(PoisonError::into_inner);
        let before = all
            .iter()
            .position(|known| known.hwnd == hwnd)
            .map(|at| all.remove(at).entries)
            .unwrap_or_default();
        let changed = changes(&before, &published.entries);
        all.push(published);
        changed
    };
    // Told once the list is let go: UI Automation may ask the provider
    // back at once, and it reads the list.
    if !listening() {
        return;
    }
    if changed.structure {
        let root: IRawElementProviderSimple = Root { hwnd }.into();
        // The root's runtime id comes from its window (it gives none of
        // its own), so none is passed.
        let _ = unsafe {
            UiaRaiseStructureChangedEvent(
                &root,
                StructureChangeType_ChildrenInvalidated,
                std::ptr::null_mut(),
                0,
            )
        };
    }
    for (key, before, after) in changed.renamed {
        let item: IRawElementProviderSimple = Item { hwnd, key }.into();
        let _ = unsafe {
            UiaRaiseAutomationPropertyChangedEvent(
                &item,
                UIA_NamePropertyId,
                &VARIANT::from(BSTR::from(before.as_str())),
                &VARIANT::from(BSTR::from(after.as_str())),
            )
        };
    }
}

/// What changed between two lists of a window, for UI Automation.
#[derive(Debug, Default, PartialEq)]
struct Changes {
    /// Items came, went or moved.
    structure: bool,
    /// Items kept whose name changed (a notice on Configurações): the key,
    /// the name before and the name now.
    renamed: Vec<(String, String, String)>,
}

fn changes(before: &[Entry], after: &[Entry]) -> Changes {
    let structure = before.len() != after.len()
        || before
            .iter()
            .zip(after)
            .any(|(old, new)| old.key != new.key);
    let renamed = after
        .iter()
        .filter_map(|new| {
            let old = before.iter().find(|old| old.key == new.key)?;
            (old.name != new.name).then(|| (new.key.clone(), old.name.clone(), new.name.clone()))
        })
        .collect();
    Changes { structure, renamed }
}

fn listening() -> bool {
    unsafe { UiaClientsAreListening() }.as_bool()
}

/// The window `hwnd` is gone, or shows nothing.
pub fn forget(hwnd: isize) {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|known| known.hwnd != hwnd);
    set_focus(hwnd, None);
}

/// The item `key` of the dock `hwnd` has the keyboard now, or none does
/// (`None`); a new one is told to UI Automation.
pub fn set_focus(hwnd: isize, key: Option<String>) {
    let changed = {
        let mut focus = FOCUS.lock().unwrap_or_else(PoisonError::into_inner);
        let before = focus
            .iter()
            .position(|(known, _)| *known == hwnd)
            .map(|at| focus.remove(at).1);
        if let Some(key) = &key {
            focus.push((hwnd, key.clone()));
        }
        before != key
    };
    if let Some(key) = key.filter(|_| changed && listening()) {
        let item: IRawElementProviderSimple = Item { hwnd, key }.into();
        let _ = unsafe { UiaRaiseAutomationEvent(&item, UIA_AutomationFocusChangedEventId) };
    }
}

/// The key of the item of the dock `hwnd` that has the keyboard.
fn focus(hwnd: isize) -> Option<String> {
    FOCUS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(known, _)| *known == hwnd)
        .map(|(_, key)| key.clone())
}

fn entries(hwnd: isize) -> Vec<Entry> {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|known| known.hwnd == hwnd)
        .map(|known| known.entries.clone())
        .unwrap_or_default()
}

/// The name of the window `hwnd` and how its items are pressed.
fn owner(hwnd: isize) -> (String, Option<Press>) {
    ENTRIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|known| known.hwnd == hwnd)
        .map_or_else(
            || (LABEL.to_owned(), None),
            |known| (known.label.clone(), known.press),
        )
}

/// The dock's name for assistive technology.
const LABEL: &str = "Dock";

/// Answers `WM_GETOBJECT` asking for UI Automation's root; `None` for any
/// other object.
pub fn get_object(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if lparam.0 as i32 != UiaRootObjectId {
        return None;
    }
    let root: IRawElementProviderSimple = Root {
        hwnd: hwnd.0 as isize,
    }
    .into();
    Some(unsafe { UiaReturnRawElementProvider(hwnd, wparam, lparam, &root) })
}

/// Nothing: a null out-pointer with success.
fn none<T>() -> Result<T> {
    Err(Error::empty())
}

fn item(hwnd: isize, key: String) -> IRawElementProviderFragment {
    Item { hwnd, key }.into()
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRawElementProviderFragmentRoot
)]
struct Root {
    hwnd: isize,
}

impl IRawElementProviderSimple_Impl for Root_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        crate::windows::panics::com_call(|| Ok(ProviderOptions_ServerSideProvider))
    }

    fn GetPatternProvider(&self, _pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        crate::windows::panics::com_call(none)
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        crate::windows::panics::com_call(|| {
            Ok(match property {
                UIA_ControlTypePropertyId => VARIANT::from(UIA_ToolBarControlTypeId.0),
                UIA_NamePropertyId => VARIANT::from(BSTR::from(owner(self.hwnd).0.as_str())),
                _ => VARIANT::default(),
            })
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        crate::windows::panics::com_call(|| unsafe {
            UiaHostProviderFromHwnd(HWND(self.hwnd as _))
        })
    }
}

impl IRawElementProviderFragment_Impl for Root_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        crate::windows::panics::com_call(|| {
            let entries = entries(self.hwnd);
            let found = match direction {
                NavigateDirection_FirstChild => entries.first(),
                NavigateDirection_LastChild => entries.last(),
                _ => None,
            };
            match found {
                Some(entry) => Ok(item(self.hwnd, entry.key.clone())),
                None => none(),
            }
        })
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        crate::windows::panics::com_call(|| {
            // The window's own: UI Automation takes it from the host.
            Ok(std::ptr::null_mut())
        })
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        crate::windows::panics::com_call(|| {
            // The window's own, from the host.
            Ok(UiaRect::default())
        })
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        crate::windows::panics::com_call(|| Ok(std::ptr::null_mut()))
    }

    fn SetFocus(&self) -> Result<()> {
        crate::windows::panics::com_call(|| Ok(()))
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        crate::windows::panics::com_call(|| Ok(self.to_interface()))
    }
}

impl IRawElementProviderFragmentRoot_Impl for Root_Impl {
    fn ElementProviderFromPoint(&self, x: f64, y: f64) -> Result<IRawElementProviderFragment> {
        crate::windows::panics::com_call(|| {
            let found = entries(self.hwnd).into_iter().find(|entry| {
                let (left, top, width, height) = entry.rect;
                x >= left && y >= top && x < left + width && y < top + height
            });
            match found {
                Some(entry) => Ok(item(self.hwnd, entry.key)),
                None => none(),
            }
        })
    }

    fn GetFocus(&self) -> Result<IRawElementProviderFragment> {
        crate::windows::panics::com_call(|| match focus(self.hwnd) {
            Some(key) => Ok(item(self.hwnd, key)),
            None => none(),
        })
    }
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider
)]
struct Item {
    hwnd: isize,
    key: String,
}

impl Item_Impl {
    fn entry(&self) -> Option<(usize, Entry)> {
        entries(self.hwnd)
            .into_iter()
            .enumerate()
            .find(|(_, entry)| entry.key == self.key)
    }
}

/// A number for the key, the same each time it is asked.
fn number_of(key: &str) -> i32 {
    // FNV-1a: stable across runs, unlike the standard hasher.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in key.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash as i32
}

impl IRawElementProviderSimple_Impl for Item_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        crate::windows::panics::com_call(|| Ok(ProviderOptions_ServerSideProvider))
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        crate::windows::panics::com_call(|| {
            if pattern == UIA_InvokePatternId {
                self.to_interface::<IInvokeProvider>().cast()
            } else {
                none()
            }
        })
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        crate::windows::panics::com_call(|| {
            let Some((_, entry)) = self.entry() else {
                return Ok(VARIANT::default());
            };
            Ok(match property {
                UIA_ControlTypePropertyId => VARIANT::from(UIA_ButtonControlTypeId.0),
                UIA_NamePropertyId => VARIANT::from(BSTR::from(entry.name.as_str())),
                UIA_AutomationIdPropertyId => VARIANT::from(BSTR::from(entry.key.as_str())),
                UIA_IsEnabledPropertyId => VARIANT::from(true),
                // A dock's items take the keyboard by its hotkey (M14); the
                // preview card's don't.
                UIA_IsKeyboardFocusablePropertyId => VARIANT::from(owner(self.hwnd).1.is_none()),
                UIA_HasKeyboardFocusPropertyId => {
                    VARIANT::from(focus(self.hwnd).as_deref() == Some(self.key.as_str()))
                }
                _ => VARIANT::default(),
            })
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        crate::windows::panics::com_call(none)
    }
}

impl IRawElementProviderFragment_Impl for Item_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        crate::windows::panics::com_call(|| {
            if direction == NavigateDirection_Parent {
                let root: IRawElementProviderFragment = Root { hwnd: self.hwnd }.into();
                return Ok(root);
            }
            let entries = entries(self.hwnd);
            let Some(index) = entries.iter().position(|entry| entry.key == self.key) else {
                return none();
            };
            let found = match direction {
                NavigateDirection_NextSibling => entries.get(index + 1),
                NavigateDirection_PreviousSibling => {
                    index.checked_sub(1).and_then(|i| entries.get(i))
                }
                _ => None,
            };
            match found {
                Some(entry) => Ok(item(self.hwnd, entry.key.clone())),
                None => none(),
            }
        })
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        crate::windows::panics::com_call(|| unsafe {
            let ids = SafeArrayCreateVector(VT_I4, 0, 2);
            if ids.is_null() {
                return none();
            }
            for (index, value) in [UiaAppendRuntimeId as i32, number_of(&self.key)]
                .into_iter()
                .enumerate()
            {
                let at = index as i32;
                SafeArrayPutElement(ids, &at, std::ptr::from_ref(&value).cast())?;
            }
            Ok(ids)
        })
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        crate::windows::panics::com_call(|| {
            Ok(match self.entry() {
                Some((_, entry)) => {
                    let (left, top, width, height) = entry.rect;
                    UiaRect {
                        left,
                        top,
                        width,
                        height,
                    }
                }
                None => UiaRect::default(),
            })
        })
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        crate::windows::panics::com_call(|| Ok(std::ptr::null_mut()))
    }

    fn SetFocus(&self) -> Result<()> {
        crate::windows::panics::com_call(|| Ok(()))
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        crate::windows::panics::com_call(|| Ok(Root { hwnd: self.hwnd }.into()))
    }
}

impl IInvokeProvider_Impl for Item_Impl {
    fn Invoke(&self) -> Result<()> {
        crate::windows::panics::com_call(|| {
            if let (_, Some(press)) = owner(self.hwnd) {
                press(self.hwnd, &self.key);
                return Ok(());
            }
            // Only this dock: each monitor has the same workspace dots.
            let key = self.key.clone();
            super::post_to(self.hwnd, move |dock| dock.press(&key));
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, name: &str) -> Entry {
        Entry {
            key: key.into(),
            name: name.into(),
            rect: (0.0, 0.0, 1.0, 1.0),
        }
    }

    #[test]
    fn the_same_list_changes_nothing() {
        let list = [entry("a", "A"), entry("b", "B")];
        assert_eq!(changes(&list, &list), Changes::default());
        assert_eq!(changes(&[], &[]), Changes::default());
    }

    #[test]
    fn items_that_come_go_or_move_change_the_structure() {
        let list = [entry("a", "A"), entry("b", "B")];
        for after in [
            vec![entry("a", "A")],
            vec![entry("a", "A"), entry("b", "B"), entry("c", "C")],
            vec![entry("b", "B"), entry("a", "A")],
            vec![entry("a", "A"), entry("c", "B")],
        ] {
            assert!(changes(&list, &after).structure, "{after:?}");
        }
        assert!(changes(&[], &list).structure);
        // Only moving on the screen is not a change of the list.
        let moved = [
            Entry {
                rect: (9.0, 9.0, 1.0, 1.0),
                ..entry("a", "A")
            },
            entry("b", "B"),
        ];
        assert_eq!(changes(&list, &moved), Changes::default());
    }

    #[test]
    fn a_name_that_changes_is_told_with_both_names() {
        let before = [entry("settings", "Configurações"), entry("b", "B")];
        let after = [
            entry("settings", "Configurações, com aviso"),
            entry("b", "B"),
            entry("c", "C"),
        ];
        let changed = changes(&before, &after);
        assert!(changed.structure);
        assert_eq!(
            changed.renamed,
            [(
                "settings".to_string(),
                "Configurações".to_string(),
                "Configurações, com aviso".to_string()
            )]
        );
    }

    #[test]
    fn an_items_number_is_stable_and_tells_keys_apart() {
        assert_eq!(number_of("about"), number_of("about"));
        assert_ne!(number_of("about"), number_of("settings"));
    }
}
