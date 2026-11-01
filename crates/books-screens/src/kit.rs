use buwiz_app::Bridge;
use nsh_core::{
    element::{Element, Interactive, ParentElement, Styled, View, view},
    geometry::{Bounds, Points},
};

pub const C_INK: &str = "text-[#1c1c1c] dark:text-[#f0f0f0]";
pub const C_MUTE: &str = "text-[#5c5c5c] dark:text-[#a0a0a0]";
pub const C_DIM: &str = "text-[#8a8a8a] dark:text-[#6e6e6e]";
pub const C_RED: &str = "text-[#c0392b] dark:text-[#ff7d75]";
pub const C_GREEN_BG: &str = "bg-[#157a48] dark:bg-[#3fcf89]";
pub const PILL_MUTE: &str = "bg-[#ececea] dark:bg-[#262626] text-[#5c5c5c] dark:text-[#a0a0a0]";
pub const PILL_DRAFT: &str = "bg-[#f8ecd2] dark:bg-[#3a2e12] text-[#a2650f] dark:text-[#f3b64c]";
pub const PILL_REV: &str = "bg-[#fbe3e0] dark:bg-[#3a1a17] text-[#c0392b] dark:text-[#ff7d75]";
pub const K_SURFACE: &str = "bg-white dark:bg-[#1f1f1f]";
pub const K_LINE: &str = "border-[#e0e0e0] dark:border-[#3a3a3a]";
pub const K_WELL: &str = "bg-[#ebebeb] dark:bg-[#333333]";
pub const K_SKEL: &str = "bg-[#efefef] dark:bg-[#2c2c2c]";

pub fn body(
    content: View,
    foot: Option<View>,
    center: bool,
    bridge: Bridge,
    fit_sent: nsh_core::element::Signal<i32>,
) -> View {
    view()
        .app_class(Some(
            format!(
                "min-h-[96px] {} {K_SURFACE}",
                if center {
                    "justify-center"
                } else {
                    "justify-between"
                }
            )
            .into(),
        ))
        .on_layout(move |bounds: &Bounds<Points>| {
            let height = bounds.size.height.0.ceil() as i32;
            if (height - fit_sent.get()).abs() >= 1 {
                fit_sent.set(height);
                let _ = bridge.fit(height as f32);
            }
        })
        .child(content)
        .child(foot.unwrap_or_else(view))
}

pub fn content() -> View {
    view().app_class(Some("gap-2 px-3 py-3".into()))
}

pub fn meta(label: impl Into<String>, right: View) -> View {
    view()
        .app_class(Some("flex-row items-center justify-between gap-2".into()))
        .child(
            nsh_core::element::text(label.into())
                .app_class(Some(
                    format!("text-[11px] uppercase tracking-wider {C_DIM}").into(),
                ))
                .number_of_lines(1),
        )
        .child(right)
}

pub fn say(label: impl Into<String>) -> View {
    view().child(
        nsh_core::element::text(label.into())
            .app_class(Some(format!("text-[13px] leading-[18px] {C_INK}").into())),
    )
}

pub fn pill(label: impl Into<String>, tone: &'static str) -> View {
    let colors = match tone {
        "draft" => PILL_DRAFT,
        "rev" => PILL_REV,
        _ => PILL_MUTE,
    };
    view()
        .app_class(Some(
            format!("rounded-full px-2 py-0.5 self-start {colors}").into(),
        ))
        .child(
            nsh_core::element::text(label.into())
                .app_class(Some(format!("text-[11px] {colors}").into()))
                .number_of_lines(1),
        )
}

pub fn act(label: impl Into<String>, primary: bool, on_press: impl Fn() + 'static) -> View {
    let colors = if primary {
        "bg-[#1c1c1c] dark:bg-[#f5f5f5]"
    } else {
        K_WELL
    };
    let ink = if primary {
        "text-white dark:text-[#1f1f1f]"
    } else {
        C_INK
    };
    view()
        .app_class(Some(
            format!("h-[28px] px-3 rounded-full items-center justify-center {colors}").into(),
        ))
        .active(|style| style.opacity(0.7))
        .on_press(on_press)
        .child(
            nsh_core::element::text(label.into())
                .app_class(Some(format!("text-[12.5px] font-medium {ink}").into()))
                .number_of_lines(1),
        )
}

pub fn foot(status: impl Into<String>, notice: impl Into<String>, action: View) -> View {
    let notice = notice.into();
    let (label, color) = if notice.is_empty() {
        (status.into(), C_MUTE)
    } else {
        (notice, C_RED)
    };
    view()
        .app_class(Some(
            format!("flex-row items-center gap-2 px-3 py-2.5 border-t {K_LINE}").into(),
        ))
        .child(
            nsh_core::element::text(label)
                .app_class(Some(format!("flex-1 text-[12px] {color}").into()))
                .number_of_lines(1),
        )
        .child(action)
}

pub fn settled(
    label: impl Into<String>,
    fit_sent: nsh_core::element::Signal<i32>,
    bridge: Bridge,
) -> View {
    let content = view()
        .app_class(Some("gap-2 px-3 py-3".into()))
        .child(
            view()
                .app_class(Some("flex-row items-center gap-2".into()))
                .child(
                    view()
                        .app_class(Some(format!("w-[18px] h-[18px] rounded-full items-center justify-center {C_GREEN_BG}").into()))
                        .child(nsh_core::element::text("✓").app_class(Some("text-[11px] font-bold text-white".into()))),
                )
                .child(
                    nsh_core::element::text(label.into())
                        .app_class(Some(format!("flex-1 text-[13px] font-medium {C_INK}").into()))
                        .number_of_lines(1),
                ),
        );
    body(content, None, true, bridge, fit_sent)
}

pub fn wait(
    error: Option<String>,
    retry: Option<impl Fn() + 'static>,
    bridge: Bridge,
    fit_sent: nsh_core::element::Signal<i32>,
) -> View {
    let content = content();
    let message = error.unwrap_or_default();
    let content = if message.is_empty() {
        content
            .child(view().app_class(Some(format!("h-[12px] w-[120px] rounded {K_SKEL}").into())))
            .child(view().app_class(Some(format!("h-[22px] w-[220px] rounded {K_SKEL}").into())))
            .child(view().app_class(Some(format!("h-[12px] w-[180px] rounded {K_SKEL}").into())))
    } else {
        content.child(
            nsh_core::element::text(message).app_class(Some(format!("text-[13px] {C_RED}").into())),
        )
    };
    let foot = retry.map(|action| foot("", "", act("Try again", false, action)));
    body(content, foot, false, bridge, fit_sent)
}
