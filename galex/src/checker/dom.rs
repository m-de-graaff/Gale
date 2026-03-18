//! DOM knowledge registry — event signatures, element types, and head properties.
//!
//! Provides compile-time knowledge about the HTML/DOM platform to enable
//! type-safe template directives (`on:`, `bind:`, `ref:`) and head blocks.

// ── Event signatures ───────────────────────────────────────────────────

/// Returns the expected event parameter type name for a DOM event.
///
/// For example, `"click"` → `"MouseEvent"`, `"input"` → `"InputEvent"`.
/// Returns `None` for unknown event names (which may still be valid
/// custom events — the checker can decide how to handle unknowns).
pub fn event_param_type(event: &str) -> Option<&'static str> {
    match event {
        // Mouse events
        "click" | "dblclick" | "mousedown" | "mouseup" | "mousemove" | "mouseenter"
        | "mouseleave" | "mouseover" | "mouseout" | "contextmenu" => Some("MouseEvent"),

        // Keyboard events
        "keydown" | "keyup" | "keypress" => Some("KeyboardEvent"),

        // Input / change events
        "input" | "change" | "beforeinput" => Some("InputEvent"),

        // Focus events
        "focus" | "blur" | "focusin" | "focusout" => Some("FocusEvent"),

        // Form events
        "submit" => Some("SubmitEvent"),
        "reset" => Some("Event"),

        // Drag events
        "drag" | "dragstart" | "dragend" | "dragover" | "dragenter" | "dragleave" | "drop" => {
            Some("DragEvent")
        }

        // Touch events
        "touchstart" | "touchend" | "touchmove" | "touchcancel" => Some("TouchEvent"),

        // Pointer events
        "pointerdown" | "pointerup" | "pointermove" | "pointerenter" | "pointerleave"
        | "pointerover" | "pointerout" | "pointercancel" => Some("PointerEvent"),

        // Scroll / wheel
        "scroll" | "scrollend" => Some("Event"),
        "wheel" => Some("WheelEvent"),

        // Animation / transition
        "animationstart" | "animationend" | "animationiteration" => Some("AnimationEvent"),
        "transitionstart" | "transitionend" | "transitionrun" | "transitioncancel" => {
            Some("TransitionEvent")
        }

        // Clipboard
        "copy" | "cut" | "paste" => Some("ClipboardEvent"),

        // Media (generic)
        "load" | "error" | "resize" | "unload" | "abort" => Some("Event"),

        _ => None,
    }
}

// ── Element type mapping ───────────────────────────────────────────────

/// Returns the DOM interface type name for an HTML element tag.
///
/// For example, `"canvas"` → `"HTMLCanvasElement"`, `"div"` → `"HTMLElement"`.
/// Unknown tags fall back to `"HTMLElement"`.
pub fn element_type(tag: &str) -> &'static str {
    match tag {
        "a" => "HTMLAnchorElement",
        "audio" => "HTMLAudioElement",
        "button" => "HTMLButtonElement",
        "canvas" => "HTMLCanvasElement",
        "details" => "HTMLDetailsElement",
        "dialog" => "HTMLDialogElement",
        "form" => "HTMLFormElement",
        "iframe" => "HTMLIFrameElement",
        "img" => "HTMLImageElement",
        "input" => "HTMLInputElement",
        "label" => "HTMLLabelElement",
        "option" => "HTMLOptionElement",
        "select" => "HTMLSelectElement",
        "table" => "HTMLTableElement",
        "textarea" => "HTMLTextAreaElement",
        "video" => "HTMLVideoElement",
        // All other standard elements use the base type
        _ => "HTMLElement",
    }
}

// ── Bind compatibility ─────────────────────────────────────────────────

/// Returns the expected inner type for a signal bound via `bind:field` on a
/// given element tag.
///
/// For example, `bind:value` on `<input>` expects `string`,
/// `bind:checked` on `<input>` expects `bool`.
pub fn bind_expected_type(_tag: &str, field: &str) -> &'static str {
    match field {
        "checked" | "disabled" | "readOnly" | "required" | "open" | "hidden" => "bool",
        "value" | "placeholder" | "src" | "href" | "alt" | "title" => "string",
        "selectedIndex" | "tabIndex" | "width" | "height" => "int",
        "volume" | "playbackRate" | "currentTime" => "float",
        _ => "string", // Default: most HTML attributes are strings
    }
}

// ── Head block properties ──────────────────────────────────────────────

/// The expected type of a `head { }` block property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadPropertyType {
    /// Property expects a string value.
    String,
    /// Property expects an object with string fields (e.g. `og: { title: "..." }`).
    StringObject,
}

/// Returns the expected type for a known `head { }` property.
///
/// Returns `None` for unknown property names (the checker can warn about these).
pub fn head_property_type(key: &str) -> Option<HeadPropertyType> {
    match key {
        // Standard <head> metadata
        "title" | "description" | "charset" | "viewport" | "canonical" | "lang" | "themeColor"
        | "author" | "generator" | "applicationName" | "referrer" | "colorScheme" | "robots"
        | "favicon" => Some(HeadPropertyType::String),

        // Open Graph / social
        "og" | "twitter" | "facebook" => Some(HeadPropertyType::StringObject),

        _ => None,
    }
}

/// All known head property names (for "did you mean?" suggestions).
#[allow(dead_code)]
pub const KNOWN_HEAD_PROPERTIES: &[&str] = &[
    "title",
    "description",
    "charset",
    "viewport",
    "canonical",
    "lang",
    "themeColor",
    "author",
    "generator",
    "applicationName",
    "referrer",
    "colorScheme",
    "robots",
    "favicon",
    "og",
    "twitter",
    "facebook",
];
