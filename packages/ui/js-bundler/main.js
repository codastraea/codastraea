import { EditorView, basicSetup } from "codemirror"
import { EditorState, EditorSelection } from "@codemirror/state"
import { python } from "@codemirror/lang-python"
import "@ui5/webcomponents/dist/Button.js";
import "@ui5/webcomponents/dist/Icon.js";
import "@ui5/webcomponents/dist/Link.js";
import "@ui5/webcomponents/dist/Menu.js";
import "@ui5/webcomponents/dist/MenuItem.js";
import "@ui5/webcomponents/dist/MenuSeparator.js";
import "@ui5/webcomponents/dist/TabContainer.js";
import "@ui5/webcomponents/dist/Tab.js";
import "@ui5/webcomponents/dist/TabSeparator.js";
import "@ui5/webcomponents/dist/Tree.js";
import "@ui5/webcomponents/dist/TreeItem.js";
import "@ui5/webcomponents/dist/TreeItemCustom.js";

export function codemirror_new(doc) {
  return new EditorView({
    extensions: [
      basicSetup,
      EditorState.readOnly.of(true),
      python()
    ],
    doc,
  });
}

export function set_selection(view, from, to) {
  view.dispatch({
    selection: EditorSelection.create([EditorSelection.range(from, to)]),
    scrollIntoView: true,
  });
}
