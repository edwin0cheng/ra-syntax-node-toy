import module from '../crate/Cargo.toml'
import CodeMirror from 'codemirror';
import Bottleneck from 'bottleneck';
import $ from "jquery";
import _ from 'bulma';

// Seems to be there is a bug in importing codemirror
// Which we have to bring the mode file directly
import 'codemirror/mode/rust/rust'
import 'codemirror/lib/codemirror.css'
import 'codemirror/theme/dracula.css'
import '../css/main.css'

const limiter = new Bottleneck({
    maxConcurrent: 1,
    minTime: 500,
    highWater: 1,
});

var editor = CodeMirror.fromTextArea(document.getElementById("code"), {
    lineNumbers: true,
    matchBrackets: true,
    theme: "dracula",
    mode: "rust",
    indentUnit: 4,
});

var elm_syntree = document.getElementById("syntax-tree");
var syntax_tree = CodeMirror.fromTextArea(elm_syntree, {
    lineNumbers: true,
    indentUnit: 4,
    readOnly: true,
});

var $mbe_tab = $("#mbe-tab #mbe-tab-content");

function mkCard(call, parent) {    
    let call_site = call.call_site;    
    let expanded = call.expanded;
    
    if(parent) {
        call_site = parent;
        expanded = call_site.replace(call.call_site, expanded);
    }    
    
    let template = `
    <article class="message is-small">
        <div class="message-header">
            <p class="is-family-code">
                ${call_site}  =>                
            </p>            
        </div>
        <div class="message-body">                
            <div class="is-family-code">${expanded}</div>
        </div>
        
    </article>`;

    let $card = $(template);
    for (var c of call.children) {
        let $c = $(`<div class="message-body children"></div>`);
        $c.append(mkCard(c, expanded));
        $card.append($c);
    }
    return $card
}

function update() {
    let res = module.parse_text_to_syntax_node(editor.getValue(), $("#macro-recursive")[0].checked);
    syntax_tree.setValue(res.syntax_nodes);
    $mbe_tab.children().empty();

    for (let i = 0; i < res.calls.length; i++) {
        let call = res.calls[i];            
        $mbe_tab.append(mkCard(call));
    }
}

update();

editor.on('changes', () => {
    limiter.schedule(() => {
        update();
    })
});

$("#macro-recursive").on('change', () => {
    limiter.schedule(() => {
        update();
    })
});

var $toolTabs = $('.tool-tab');
var $toolLinks = $(".tool-tabs li");

function setActive($e) {
    $toolLinks.removeClass('is-active');
    $e.addClass('is-active');

    $toolTabs.hide();
    $("#" + $e.data("tab")).show();
}

function reset() {
    setActive($('.tool-tabs li.is-active'));
}

$toolLinks.on('click', function () {
    setActive($(this));
});


reset();