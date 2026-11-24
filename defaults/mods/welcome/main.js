// The screen with no space open: logo, name, two actions and the shortcuts (read from keys.json).
// The app paints it in the main area only while there is no space; it takes no props.
import { View, div, image } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { run, openOverlay, theme, keys, tr, style } from "herdr";
import { shortcuts } from "./model.js";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

export default class Welcome extends View {
  button(t, id, text, primary, action) {
    return h_flex().id(id).h(34).px(16).items_center().rounded(R(8)).border_1().border_color(primary ? t.solid : t.input).bg(primary ? t.solid : t.raised)
      .text_color(primary ? t.solid_foreground : t.foreground).text_size(12.8).font_medium().cursor_pointer().hover((s) => s.opacity(0.88)).on_click(action).child(text);
  }

  key(t, part) {
    return div().min_w(22).h(22).px(6).flex().items_center().justify_center().rounded(R(5)).border_1().border_color(t.border).bg(t.raised).text_size(11.5).text_color(t.muted).child(part);
  }

  row(t, s) {
    return h_flex().w_full().items_center().gap(16)
      .child(div().flex_1().min_w(0).text_size(12.5).text_color(t.muted).truncate().child(s.label))
      .child(h_flex().gap(4).flex_none().children(s.parts.map((p) => this.key(t, p))));
  }

  render(_cx) {
    const t = theme();
    const list = shortcuts(keys(), tr);
    return div().size_full().flex().items_center().justify_center().bg(t.background).rounded(R(10)).border_1().border_color(t.border).text_color(t.foreground)
      .child(v_flex().w_full().max_w(380).p(24).gap(24).items_center()
        .child(v_flex().gap(10).items_center()
          .child(image("frame/welcome/sunna.png").size(88).rounded(R(20)))
          .child(div().text_size(30).font_semibold().child("Sunna"))
          .child(div().text_size(13).text_color(t.muted).text_center().child(tr("welcome.empty_subtitle"))))
        .child(h_flex().gap(10).justify_center().flex_wrap()
          .child(this.button(t, "wl-add", tr("welcome.empty_add_project"), false, () => openOverlay({ mod: "add-project", kind: "popup", at: "pointer", width: 300, height: 160, props: { step: "menu" } })))
          .child(this.button(t, "wl-new", tr("welcome.empty_new_space"), true, () => run("space.new", {}))))
        .when(list.length > 0, (el) => el.child(v_flex().w_full().gap(10).pt(4).children(list.map((s) => this.row(t, s))))));
  }
}
