import init, { sanitize_text_wasm } from './pkg/ghostmark_wasm.js';

let wasmLoaded = false;

async function run() {
  try {
    await init();
    wasmLoaded = true;
    setStatus('WASM Engine Loaded', 'ready');
  } catch (e) {
    setStatus('Failed to load WASM', 'error');
    console.error(e);
  }
}

function appendUserMessage(text) {
  const chatArea = document.getElementById('chatArea');
  const msgDiv = document.createElement('div');
  msgDiv.className = 'message user';
  
  // Truncate for display if it's too long
  const displayText = text.length > 200 ? text.substring(0, 200) + '...' : text;
  
  msgDiv.innerHTML = `<div class="message-content">${escapeHtml(displayText)}</div>`;
  chatArea.appendChild(msgDiv);
  chatArea.scrollTop = chatArea.scrollHeight;
}

function appendAssistantMessage(html) {
  const chatArea = document.getElementById('chatArea');
  const msgDiv = document.createElement('div');
  msgDiv.className = 'message assistant';
  msgDiv.innerHTML = `
    <div class="message-content">
      <div class="assistant-icon">👻</div>
      <div class="assistant-text">${html}</div>
    </div>
  `;
  chatArea.appendChild(msgDiv);
  chatArea.scrollTop = chatArea.scrollHeight;
}

document.getElementById('scrubBtn').addEventListener('click', async () => {
  if (!wasmLoaded) return;
  
  const inputEl = document.getElementById('inputText');
  const input = inputEl.value;
  if (!input) return;

  // 1. Show user message
  appendUserMessage(input);
  
  // 2. Clear input & reset height
  inputEl.value = '';
  inputEl.style.height = '22px';

  try {
    const originalLen = input.length;
    const t0 = performance.now();
    
    // Call the Rust WASM module
    const cleaned = sanitize_text_wasm(input);
    
    const t1 = performance.now();
    const removed = originalLen - cleaned.length;
    
    // Copy back to clipboard
    await navigator.clipboard.writeText(cleaned);
    
    // 3. Show assistant response
    const responseHtml = `
      <div style="margin-top: 4px;">
        <span style="color: #10b981; font-weight: 500;">✓ Cleaned ${removed} hidden characters</span><br>
        <span style="color: #8e8ea0; font-size: 13px;">Processed in ${(t1 - t0).toFixed(2)}ms. The clean text is copied to your clipboard.</span>
      </div>
    `;
    appendAssistantMessage(responseHtml);
    
    setStatus(`Scrubbed ${removed} chars`, 'success');
  } catch (err) {
    setStatus('Error during scrubbing.', 'error');
    appendAssistantMessage(`<span style="color: #ef4444;">Error processing text. Check console.</span>`);
    console.error(err);
  }
});

// Allow hitting Enter to submit
document.getElementById('inputText').addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    document.getElementById('scrubBtn').click();
  }
});

function setStatus(msg, state) {
  const statusEl = document.getElementById('status');
  const statusBox = document.getElementById('statusBox');
  
  statusEl.innerText = msg;
  statusBox.className = `status state-${state}`;
}

function escapeHtml(unsafe) {
    return unsafe
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}

run();
