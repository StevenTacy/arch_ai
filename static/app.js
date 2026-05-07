// Auto-resize textarea
let ta = document.getElementById("message-input");
ta.addEventListener("input", function () {
  this.style.height = "auto";
  this.style.height = Math.min(this.scrollHeight, 160) + "px";
});

// Enter = submit, Shift+Enter = newline
document.addEventListener("keydown", function (e) {
  if (e.target.id === "message-input" && e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    document.getElementById("chat-form").requestSubmit();
  }
});

let _welcomeHTML = document.getElementById("messages").innerHTML;
let _sessionHistoryAdded = false;
let _swapHistoryDone = false;

// Clear textarea on submit; reset per-request guard
document.addEventListener("htmx:beforeRequest", function (e) {
  if (e.detail.elt.id === "chat-form") {
    ta.value = "";
    ta.style.height = "auto";
    _swapHistoryDone = false;
  }
});

// After htmx injects new content:
//  - render markdown in .ai-bubble[data-md] elements
//  - scroll messages to bottom
//  - add first message of session to sidebar history (once per session, once per swap)
document.addEventListener("htmx:afterSwap", function (e) {
  if (e.detail.target.id !== "messages") return;

  e.detail.target
    .querySelectorAll(".ai-bubble[data-md]:not([data-rendered])")
    .forEach(function (el) {
      el.innerHTML = marked.parse(el.textContent || "");
      el.setAttribute("data-rendered", "1");
    });
  e.detail.target.scrollTop = e.detail.target.scrollHeight;

  if (_swapHistoryDone || _sessionHistoryAdded) return;
  _swapHistoryDone = true;
  _sessionHistoryAdded = true;

  let userBubbles = e.detail.target.querySelectorAll(".user-bubble");
  let lastBubble = userBubbles[userBubbles.length - 1];
  if (!lastBubble) return;

  let msgText = lastBubble.textContent.trim();
  let examplesEl = document.getElementById("sidebar-examples");
  if (examplesEl) examplesEl.style.display = "none";

  let historyEl = document.getElementById("sidebar-history");
  if (!historyEl) return;
  if (!historyEl.querySelector(".sidebar-section")) {
    let header = document.createElement("div");
    header.className = "sidebar-section";
    header.textContent = "History";
    historyEl.appendChild(header);
  }
  let sessionId = document.getElementById("session-id-input").value;
  let item = document.createElement("div");
  item.className = "sidebar-history-item";
  item.tabIndex = 0;
  item.dataset.sessionId = sessionId;
  item.textContent = msgText.length > 55 ? msgText.slice(0, 52) + "…" : msgText;
  item.addEventListener("click", restoreSession);
  item.addEventListener("keydown", function (e) {
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); restoreSession.call(this); }
  });
  historyEl.appendChild(item);
});

// Restore a previous session from Redis into #messages
function restoreSession() {
  let sessionId = this.dataset.sessionId;
  if (!sessionId) return;
  _sessionHistoryAdded = true;
  document.getElementById("session-id-input").value = sessionId;
  htmx.ajax("GET", "/session/" + sessionId, {
    target: "#messages",
    swap: "innerHTML",
  });
}

// Fill textarea from sidebar example click
function fillExample(el) {
  ta.value = el.textContent.trim();
  ta.style.height = "auto";
  ta.style.height = Math.min(ta.scrollHeight, 160) + "px";
  ta.focus();
}

// New conversation: clear chat + session, keep history
document.querySelector(".new-chat-btn").addEventListener("click", function () {
  document.getElementById("messages").innerHTML = _welcomeHTML;
  document.getElementById("session-id-input").value = "";
  ta.value = "";
  ta.style.height = "auto";
  _sessionHistoryAdded = false;

  let historyEl = document.getElementById("sidebar-history");
  let examplesEl = document.getElementById("sidebar-examples");
  let hasHistory =
    historyEl && historyEl.querySelector(".sidebar-history-item");
  if (examplesEl) examplesEl.style.display = hasHistory ? "none" : "";
});
