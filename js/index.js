import module from '../crate/Cargo.toml'
import CodeMirror from 'codemirror';
import Bottleneck from 'bottleneck';

// Seems to be there is a bug in importing codemirror
// Which we have to bring the mode file directly
import 'codemirror/mode/rust/rust'
import 'codemirror/lib/codemirror.css'
import 'codemirror/theme/dracula.css'
import '../css/main.css'

const limiter = new Bottleneck({
    maxConcurrent: 1,
    minTime: 500
});

var editor = CodeMirror.fromTextArea(document.getElementById("code"), {
    lineNumbers: true,
    matchBrackets: true,
    theme: "dracula",
    mode: "rust",
    indentUnit: 4,
});

var elm_syntree = document.getElementById("syntax_tree");
var syntax_tree = CodeMirror.fromTextArea(elm_syntree, {
    lineNumbers: true,
    indentUnit: 4,
    readOnly: true,
});

syntax_tree.setValue(module.parse_text_to_syntax_node(editor.getValue()));


editor.on('changes', () => {
    limiter.schedule(() => {
        syntax_tree.setValue(module.parse_text_to_syntax_node(editor.getValue()));
    })
});

