// A theme as a small preview card (the same drawing as Settings > Look, which a frame can't import from here).
import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";

export function themeCard(t, entry, { selected = false, onClick, size = { w: 96, h: 60 }, id } = {}) {
  const c = entry.colors, st = entry.style ?? {};
  const dot = (color) => div().w(5).h(5).rounded(3).bg(color).flex_none();
  const bar = (color, opacity, w) => div().h(4).w(Math.round((size.w - 32) * w)).rounded(2).bg(color).opacity(opacity);
  const thumb = h_flex().relative().w(size.w).h(size.h).flex_none().rounded(st.pane_radius ?? 6).overflow_hidden().bg(c.background)
    .child(div().w(16).h_full().flex_none().bg(c.chrome))
    .child(v_flex().flex_1().h_full()
      .child(h_flex().w_full().h(10).flex_none().px(4).gap(3).items_center().bg(c.chrome).child(dot(c.ansi1)).child(dot(c.ansi2)))
      .child(v_flex().flex_1().p(5).gap(4).child(bar(c.terminal_foreground, 0.7, 0.8)).child(bar(c.ansi2, 0.45, 0.55)).child(bar(c.ansi4, 0.6, 0.7))))
    .child(div().absolute().right(4).bottom(4).px(4).py(1).rounded(st.radius ?? 6).bg(c.accent).text_color(c.accent_foreground).text_size(9).child("Aa"));
  const label = entry.name.replace(/-/g, " ");
  return v_flex().id(id ?? `theme-${entry.name}`).gap(4).p(4).rounded(10)
    .when(selected, (el) => el.border_2().border_color(t.accent))
    .when(!selected, (el) => el.border_1().border_color(t.border))
    .cursor_pointer().hover((s) => s.bg(t.raised)).on_click(onClick)
    .child(thumb)
    .child(div().w(size.w).truncate().text_size(11.5).text_color(selected ? t.foreground : t.muted).child(label));
}
