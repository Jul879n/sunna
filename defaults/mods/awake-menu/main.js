import { View, div, svg } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
import { theme, settings, run, closeOverlay, props, tr } from "herdr";

const MODES = [["on", "awake.menu.on"], ["auto", "awake.menu.auto"], ["off", "awake.menu.off"]];

export default class AwakeMenu extends View {
  row(id, label, current) {
    const t = theme();
    return h_flex().id(id).h(31).px(9).gap(10).rounded(7).items_center().flex_none().text_color(t.foreground)
      .hover(s => s.bg(t.selected))
      .on_click(() => { closeOverlay(props().overlay); run("app.keep_awake", { mode: id }); })
      .child(current ? svg("mods/workspace-chrome/check.svg").size(14) : div().w(14))
      .child(div().flex_1().child(label));
  }
  render() {
    const t = theme();
    const mode = settings().keep_awake ?? "off";
    return v_flex().w_full().h_full().bg(t.popover).p(5).text_size(12.8)
      .children(MODES.map(([id, key]) => this.row(id, tr(key), mode === id)));
  }
}
