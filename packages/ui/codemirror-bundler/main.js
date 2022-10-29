import { EditorView, basicSetup } from "codemirror"
import { python } from "@codemirror/lang-python"

export function codemirror_new(parent) {
  new EditorView({
    extensions: [basicSetup, python()],
    parent
  })
}
