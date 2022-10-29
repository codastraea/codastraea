import { EditorView, basicSetup } from "codemirror"
import { EditorState } from "@codemirror/state"
import { python } from "@codemirror/lang-python"

export function codemirror_new(doc) {
  return new EditorView({
    extensions: [
      basicSetup,
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      python()
    ],
    doc,
  });
}

export function codemirror_dom(editor) {
  return editor.dom;
}
