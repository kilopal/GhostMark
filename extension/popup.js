import init, { sanitize_text_wasm, strip_image_bytes_wasm } from './pkg/ghostmark_wasm.js';

let wasmLoaded = false;

async function run() {
  try {
    await init();
    wasmLoaded = true;
    setStatus('WASM Engine Loaded', 'ready');
    
    // Load chat history
    chrome.storage.local.get(['chatHistory'], (result) => {
      if (result.chatHistory) {
        document.getElementById('chatArea').innerHTML = result.chatHistory;
        const chatArea = document.getElementById('chatArea');
        chatArea.scrollTop = chatArea.scrollHeight;
      }
    });
  } catch (e) {
    setStatus('Failed to load WASM', 'error');
    console.error(e);
  }
}

// Message listener for progress updates from background.js
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'progress') {
    setStatus(request.message, request.status === 'error' ? 'error' : 'ready');
  }
});

// Save chat history
function saveChatHistory() {
  const html = document.getElementById('chatArea').innerHTML;
  chrome.storage.local.set({ chatHistory: html });
}

// Robust clipboard copy fallback
async function copyToClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (err) {
    const el = document.createElement('textarea');
    el.value = text;
    el.setAttribute('readonly', '');
    el.style.position = 'absolute';
    el.style.left = '-9999px';
    document.body.appendChild(el);
    el.select();
    document.execCommand('copy');
    document.body.removeChild(el);
  }
}

// ==========================================
// IMAGE PROCESSING
// ==========================================

async function processImageFile(file) {
  if (!wasmLoaded) return;
  
  // Update UI to show upload
  appendUserMessage(`📎 Uploaded Image: ${file.name} (${(file.size / 1024).toFixed(1)} KB)`);
  
  try {
    const t0 = performance.now();
    
    // Read file as ArrayBuffer
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    
    // Call Rust WASM to strip C2PA chunks in-memory
    const cleanedBytes = strip_image_bytes_wasm(uint8Array);
    
    const t1 = performance.now();
    const removedBytes = uint8Array.length - cleanedBytes.length;
    
    // Create new blob and auto-download
    const blob = new Blob([cleanedBytes], { type: file.type });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    // Insert _ghostmark before the extension
    const nameParts = file.name.split('.');
    const ext = nameParts.pop();
    a.download = `${nameParts.join('.')}_ghostmark.${ext}`;
    a.click();
    
    setTimeout(() => URL.revokeObjectURL(url), 1000);

    const responseHtml = `
      <div style="margin-top: 4px;">
        <span style="color: #10b981; font-weight: 500;">✓ Image Metadata Stripped</span><br>
        <span style="color: #8e8ea0; font-size: 13px;">Removed ${removedBytes} bytes of C2PA/Exif tracking data in ${(t1 - t0).toFixed(2)}ms.<br>The clean image has been downloaded.</span>
      </div>
    `;
    appendAssistantMessage(responseHtml);
    setStatus(`Stripped ${removedBytes} bytes`, 'success');
    
  } catch (err) {
    setStatus('Error processing image', 'error');
    appendAssistantMessage(`<span style="color: #ef4444;">Error processing image: ${err}</span>`);
    console.error(err);
  }
}

// Manual Upload Button
document.getElementById('uploadBtn').addEventListener('click', () => {
  document.getElementById('fileInput').click();
});

document.getElementById('fileInput').addEventListener('change', (e) => {
  if (e.target.files.length > 0) {
    processImageFile(e.target.files[0]);
    e.target.value = ''; // reset
  }
});

// Drag and Drop Overlays
const dragOverlay = document.getElementById('dragOverlay');

window.addEventListener('dragover', (e) => {
  e.preventDefault();
  dragOverlay.style.display = 'flex';
});

dragOverlay.addEventListener('dragleave', (e) => {
  e.preventDefault();
  dragOverlay.style.display = 'none';
});

dragOverlay.addEventListener('drop', (e) => {
  e.preventDefault();
  dragOverlay.style.display = 'none';
  
  if (e.dataTransfer.files.length > 0) {
    const file = e.dataTransfer.files[0];
    if (file.type.startsWith('image/')) {
      processImageFile(file);
    } else {
      appendAssistantMessage(`<span style="color: #ef4444;">Please drop a valid image file.</span>`);
    }
  }
});

// ==========================================
// TEXT PROCESSING
// ==========================================

function appendUserMessage(text) {
  const chatArea = document.getElementById('chatArea');
  const msgDiv = document.createElement('div');
  msgDiv.className = 'message user';
  
  // Truncate for display if it's too long
  const displayText = text.length > 200 ? text.substring(0, 200) + '...' : text;
  
  msgDiv.innerHTML = `<div class="message-content">${escapeHtml(displayText)}</div>`;
  chatArea.appendChild(msgDiv);
  chatArea.scrollTop = chatArea.scrollHeight;
  saveChatHistory();
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
  saveChatHistory();
}

document.getElementById('scrubBtn').addEventListener('click', async () => {
  if (!wasmLoaded) return;
  
  const inputEl = document.getElementById('inputText');
  const input = inputEl.value;
  if (!input) return;

  const aggressive = document.getElementById('aggressiveScrub').checked;

  // 1. Show user message
  appendUserMessage(input);
  
  // 2. Clear input & reset height
  inputEl.value = '';
  inputEl.style.height = '22px';

  try {
    const originalLen = input.length;
    const t0 = performance.now();
    
    if (aggressive) {
      // 1. First, run the fast WASM normalization and Unicode scrub
      const wasmCleaned = sanitize_text_wasm(input, false);
      
      setStatus('Initializing Neural Network...', 'ready');
      const llmT0 = performance.now();
      
      // 2. Send to background script for LLM paraphrasing
      chrome.runtime.sendMessage({ action: 'paraphrase', text: wasmCleaned }, async (response) => {
        if (chrome.runtime.lastError) {
           console.error("Runtime error:", chrome.runtime.lastError);
           setStatus('Error connecting to AI engine', 'error');
           appendAssistantMessage(`<span style="color: #ef4444;">Error: Could not connect to local AI engine. Make sure the extension is reloaded.</span>`);
           return;
        }

        if (response && response.success) {
          const llmT1 = performance.now();
          const finalCleaned = response.text;
          
          await copyToClipboard(finalCleaned);
          
          const responseHtml = `
            <div style="margin-top: 4px;">
              <span style="color: #10b981; font-weight: 500;">✓ Statistical Watermark Destroyed</span><br>
              <span style="color: #8e8ea0; font-size: 13px;">Rewrote text using local neural network in ${((llmT1 - llmT0) / 1000).toFixed(1)}s. Copied to clipboard!</span>
            </div>
            <div style="margin-top: 8px; padding: 10px; background: rgba(0,0,0,0.2); border-radius: 6px; font-size: 14px; color: #ececec; border-left: 2px solid #10b981;">
              ${escapeHtml(finalCleaned)}
            </div>
          `;
          appendAssistantMessage(responseHtml);
          setStatus(`Paraphrased successfully`, 'success');
        } else {
          setStatus('AI Engine Error', 'error');
          appendAssistantMessage(`<span style="color: #ef4444;">AI Engine Error: ${response?.error || 'Unknown error'}</span>`);
        }
      });
      
      return; // We wait for the async callback
      
    } else {
      // Regular WASM fast scrub (Layer A)
      const cleaned = sanitize_text_wasm(input, false);
      const t1 = performance.now();
      const removed = originalLen - cleaned.length;
      
      await copyToClipboard(cleaned);
      
      const responseHtml = `
        <div style="margin-top: 4px;">
          <span style="color: #10b981; font-weight: 500;">✓ Cleaned ${removed > 0 ? removed : 0} hidden characters</span><br>
          <span style="color: #8e8ea0; font-size: 13px;">Processed in ${(t1 - t0).toFixed(2)}ms. Copied to clipboard.</span>
        </div>
      `;
      appendAssistantMessage(responseHtml);
      setStatus(`Scrubbed ${removed > 0 ? removed : '0'} chars`, 'success');
    }
    
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

// Auto-resize textarea like ChatGPT
const tx = document.getElementById('inputText');
tx.setAttribute('style', 'height:' + (tx.scrollHeight) + 'px;overflow-y:hidden;');
tx.addEventListener("input", function OnInput() {
  this.style.height = '22px';
  this.style.height = (this.scrollHeight) + 'px';
  if(this.scrollHeight > 150) {
    this.style.overflowY = 'auto';
  } else {
    this.style.overflowY = 'hidden';
  }
}, false);
