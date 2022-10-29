import {EditorView, basicSetup} from "codemirror"
import {javascript} from "@codemirror/lang-python"

let editor = new EditorView({
  extensions: [basicSetup, python()],
  parent: document.body
})
