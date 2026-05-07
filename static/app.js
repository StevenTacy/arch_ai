// Auto-resize textarea
const ta = document.getElementById('message-input');
ta.addEventListener('input', function () {
    this.style.height = 'auto';
    this.style.height = Math.min(this.scrollHeight, 160) + 'px';
});

// Enter = submit, Shift+Enter = newline
document.addEventListener('keydown', function (e) {
    if (e.target.id === 'message-input' && e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        document.getElementById('chat-form').requestSubmit();
    }
});

// Clear textarea immediately on submit (before server responds)
document.addEventListener('htmx:beforeRequest', function (e) {
    if (e.detail.elt.id === 'chat-form') {
        ta.value = '';
        ta.style.height = 'auto';
    }
});

// After htmx injects new content:
//  - render markdown in .ai-bubble[data-md] elements
//  - scroll messages to bottom
document.addEventListener('htmx:afterSwap', function (e) {
    if (e.detail.target.id === 'messages') {
        e.detail.target.querySelectorAll('.ai-bubble[data-md]:not([data-rendered])').forEach(function (el) {
            el.innerHTML = marked.parse(el.textContent || '');
            el.setAttribute('data-rendered', '1');
        });
        e.detail.target.scrollTop = e.detail.target.scrollHeight;
    }
});

// Fill textarea from sidebar example click
function fillExample(el) {
    ta.value = el.textContent.trim();
    ta.style.height = 'auto';
    ta.style.height = Math.min(ta.scrollHeight, 160) + 'px';
    ta.focus();
}
