import init, { sanitize_text_wasm, strip_image_bytes_wasm } from './pkg/ghostmark_wasm.js';
import { pipeline, env } from '@huggingface/transformers';

// ==========================================
// CONFIGURE TRANSFORMERS.JS FOR CHROME EXTENSION
// ==========================================

// Disable local model checks (we always fetch from HF Hub)
env.allowLocalModels = false;

// Use single-threaded WASM (no workers = no CSP blob: issues)
env.backends.onnx.wasm.numThreads = 1;
env.backends.onnx.wasm.proxy = false;

// Provide the local extension path for ONNX WASM binaries
env.backends.onnx.wasm.wasmPaths = chrome.runtime.getURL('dist/assets/');

let paraphraser = null;
let pipelinePromise = null;
let wasmLoaded = false;

// ==========================================
// PARAPHRASE ENGINE (Transformers.js)
// ==========================================

async function getParaphraser() {
  if (paraphraser) return paraphraser;
  if (pipelinePromise) return pipelinePromise;

  pipelinePromise = (async () => {
    try {
      setStatus('Downloading AI model...', 'ready');

      // Use Llama-3.2-1B-Instruct (1.2 Billion parameters) via ONNX
      // This is a large model (~800MB - 1.5GB quantized) and will be slower on CPU
      const pipe = await pipeline('text-generation', 'onnx-community/Llama-3.2-1B-Instruct', {
        dtype: 'q8', // Fast on both WebGPU and WASM fallback
        device: navigator.gpu ? 'webgpu' : 'wasm',
        progress_callback: (progress) => {
          if (progress.status === 'progress' && progress.progress) {
            setStatus(`Downloading 1B Model: ${Math.round(progress.progress)}%`, 'ready');
          } else if (progress.status === 'ready') {
            setStatus('AI model loaded', 'success');
          }
        }
      });

      paraphraser = pipe;
      setStatus('Ready', 'success');
      return pipe;
    } catch (err) {
      pipelinePromise = null;
      throw err;
    }
  })();

  return pipelinePromise;
}

// ==========================================
// STARTUP
// ==========================================

async function run() {
  try {
    // 1. Load WASM engine (always works, no GPU needed)
    await init({ module_or_path: chrome.runtime.getURL('pkg/ghostmark_wasm_bg.wasm') });
    wasmLoaded = true;
    setStatus('Ready', 'success');

    // 2. Load chat history
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

// Message listener removed since everything is local now

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
      <div class="result-avatar">
        <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 2C8.13 2 5 5.13 5 9c0 1.74.56 3.35 1.5 4.66V22l2.5-2 2 2 2-2 2 2 2-2 2.5 2v-8.34A6.96 6.96 0 0019 9c0-3.87-3.13-7-7-7z" fill="currentColor" stroke="none"/>
          <circle cx="9.5" cy="9" r="1.5" fill="var(--bg-base)"/>
          <circle cx="14.5" cy="9" r="1.5" fill="var(--bg-base)"/>
        </svg>
      </div>
      <div class="result-content">
        <div class="result-output">Image Metadata Stripped.</div>
        <div class="result-meta">
          <div class="status-dot success"></div>
          <span>Removed ${removedBytes} bytes of C2PA/Exif tracking data in ${(t1 - t0).toFixed(2)}ms. Downloaded!</span>
        </div>
      </div>
    `;
    appendAssistantMessage(responseHtml);
    setStatus(`Stripped ${removedBytes} bytes`, 'success');
    
  } catch (err) {
    setStatus('Error processing image', 'error');
    appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Error processing image</div><div class="result-meta"><div class="status-dot error"></div><span>${escapeHtml(err.toString())}</span></div></div>`);
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
      appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Invalid file type</div><div class="result-meta"><div class="status-dot error"></div><span>Please drop a valid image file (PNG, JPEG, or WebP).</span></div></div>`);
    }
  }
});

// ==========================================
// TEXT PROCESSING
// ==========================================

function appendUserMessage(text) {
  const chatArea = document.getElementById('chatArea');
  const entryDiv = document.createElement('div');
  entryDiv.className = 'entry-user';
  
  // Truncate for display if it's too long
  const displayText = text.length > 200 ? text.substring(0, 200) + '...' : text;
  
  entryDiv.innerHTML = `<div class="user-quote">${escapeHtml(displayText)}</div>`;
  chatArea.appendChild(entryDiv);
  chatArea.scrollTop = chatArea.scrollHeight;
  saveChatHistory();
}

function appendAssistantMessage(html) {
  const chatArea = document.getElementById('chatArea');
  const entryDiv = document.createElement('div');
  entryDiv.className = 'entry-result';
  entryDiv.innerHTML = html;
  chatArea.appendChild(entryDiv);
  chatArea.scrollTop = chatArea.scrollHeight;
  saveChatHistory();
}

document.getElementById('scrubBtn').addEventListener('click', async () => {
  if (!wasmLoaded) return;
  
  const inputEl = document.getElementById('inputText');
  const input = inputEl.value;
  if (!input) return;

  const aggressive = document.getElementById('aggressiveScrub').checked;
  const grammar = document.getElementById('grammarScrub').checked;

  // 1. Show user message
  appendUserMessage(input);
  
  // 2. Clear input & reset height
  inputEl.value = '';
  inputEl.style.height = '22px';

  try {
    const originalLen = input.length;
    const t0 = performance.now();
    
    if (aggressive || grammar) {
      // 1. First, run the fast WASM normalization and Unicode scrub
      const wasmCleaned = sanitize_text_wasm(input, false);
      
      const llmT0 = performance.now();
      
      try {
        const ollamaMode = document.getElementById('ollamaMode')?.checked;
        const ollamaModel = document.getElementById('ollamaModel')?.value || 'llama3';
        
        let pipe = null;
        if (!ollamaMode) {
          pipe = await getParaphraser();
        }
        setStatus(grammar ? 'Checking grammar...' : 'Rewriting text...', 'ready');
        
        // Use larger chunks if using Ollama since it has more memory/VRAM
        const maxChunkLen = ollamaMode ? 4000 : 400;
        const chunks = [];
        for (let i = 0; i < wasmCleaned.length; i += maxChunkLen) {
          chunks.push(wasmCleaned.slice(i, i + maxChunkLen));
        }

        const rewrittenParts = [];
        for (const chunk of chunks) {
          const sysPrompt = grammar 
            ? 'You are an expert proofreader. Fix any grammatical, spelling, or punctuation errors in the user\'s text. Do not rewrite or paraphrase. Output only the corrected text.'
            : 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. You MUST preserve the exact same meaning, names, genders, and pronouns (he/she/they) as the original. Output only the rewritten text.';
            
          let reply = '';
          if (ollamaMode) {
            const response = await fetch('http://localhost:11434/api/generate', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                model: ollamaModel,
                system: sysPrompt,
                prompt: chunk,
                stream: false,
                options: {
                  temperature: grammar ? 0.2 : 0.6
                }
              })
            });
            if (!response.ok) throw new Error('Ollama server not responding. Is it running?');
            const data = await response.json();
            reply = data.response;
          } else {
            const messages = [
              { role: 'system', content: sysPrompt },
              { role: 'user', content: chunk }
            ];
            
            const result = await pipe(messages, {
              max_new_tokens: 512,
              temperature: grammar ? 0.2 : 0.6, // Lowered to 0.6 to prevent pronoun flipping
              top_p: 0.9,
              repetition_penalty: 1.05,
              do_sample: true
            });
            // Extract the assistant's reply from the generated output
            const generatedText = result[0].generated_text;
            reply = generatedText[generatedText.length - 1].content;
          }
          rewrittenParts.push(reply);
        }

        // 2. Run the algorithmic Homoglyph Perturbation on the generated text
        const finalCleaned = grammar ? rewrittenParts.join(' ') : applyHomoglyphs(rewrittenParts.join(' '));
        const llmT1 = performance.now();
        
        await copyToClipboard(finalCleaned);
        
        const title = grammar ? 'Grammar Corrected' : 'Statistical Watermark Destroyed';
        
        const responseHtml = `
          <div class="result-avatar">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M12 2C8.13 2 5 5.13 5 9c0 1.74.56 3.35 1.5 4.66V22l2.5-2 2 2 2-2 2 2 2-2 2.5 2v-8.34A6.96 6.96 0 0019 9c0-3.87-3.13-7-7-7z" fill="currentColor" stroke="none"/>
              <circle cx="9.5" cy="9" r="1.5" fill="var(--bg-base)"/>
              <circle cx="14.5" cy="9" r="1.5" fill="var(--bg-base)"/>
            </svg>
          </div>
          <div class="result-content">
            <div class="result-output">${escapeHtml(finalCleaned)}</div>
            <div class="result-meta">
              <div class="status-dot success"></div>
              <span>${title} (${((llmT1 - llmT0) / 1000).toFixed(1)}s) - Copied!</span>
            </div>
          </div>
        `;
        appendAssistantMessage(responseHtml);
        setStatus('Processed successfully', 'success');
      } catch (err) {
        setStatus('AI Engine Error', 'error');
        appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">AI Engine Error</div><div class="result-meta"><div class="status-dot error"></div><span>${escapeHtml(err.toString())}</span></div></div>`);
        console.error("AI Error:", err);
      }
      
      return;
      
    } else {
      // Regular WASM fast scrub (Layer A)
      const cleaned = sanitize_text_wasm(input, false);
      const t1 = performance.now();
      const removed = originalLen - cleaned.length;
      
      await copyToClipboard(cleaned);
      
      const responseHtml = `
        <div class="result-avatar">
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 2C8.13 2 5 5.13 5 9c0 1.74.56 3.35 1.5 4.66V22l2.5-2 2 2 2-2 2 2 2-2 2.5 2v-8.34A6.96 6.96 0 0019 9c0-3.87-3.13-7-7-7z" fill="currentColor" stroke="none"/>
            <circle cx="9.5" cy="9" r="1.5" fill="var(--bg-base)"/>
            <circle cx="14.5" cy="9" r="1.5" fill="var(--bg-base)"/>
          </svg>
        </div>
        <div class="result-content">
          <div class="result-output">Cleaned ${removed > 0 ? removed : 0} hidden characters.</div>
          <div class="result-meta">
            <div class="status-dot success"></div>
            <span>Fast WASM Scrub (${(t1 - t0).toFixed(2)}ms) - Copied!</span>
          </div>
        </div>
      `;
      appendAssistantMessage(responseHtml);
      setStatus(`Scrubbed ${removed > 0 ? removed : '0'} chars`, 'success');
    }
    
  } catch (err) {
    setStatus('Error during scrubbing.', 'error');
    appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Processing Error</div><div class="result-meta"><div class="status-dot error"></div><span>Check console for details.</span></div></div>`);
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

// Complex Mathematical Algorithm for Text Perturbation (Defeats Tokenizers)
function applyHomoglyphs(text) {
  const glyphs = {
    'a': 'а', // U+0430
    'c': 'с', // U+0441
    'e': 'е', // U+0435
    'o': 'о', // U+043E
    'p': 'р', // U+0440
    'x': 'х', // U+0445
    'y': 'у', // U+0443
    'A': 'А', // U+0410
    'C': 'С', // U+0421
    'E': 'Е', // U+0415
    'O': 'О', // U+041E
    'P': 'Р', // U+0420
    'X': 'Х', // U+0425
  };

  let result = '';
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    // 15% chance to swap with a Cyrillic homoglyph
    if (glyphs[char] && Math.random() < 0.15) {
      result += glyphs[char];
    } else {
      result += char;
    }
    
    // 5% chance to inject an invisible zero-width non-joiner 
    // (but not at spaces to avoid word boundary issues)
    if (char !== ' ' && Math.random() < 0.05) {
      result += '\u200C'; 
    }
  }
  return result;
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

// Mutually exclusive toggles
document.getElementById('aggressiveScrub').addEventListener('change', (e) => {
  if (e.target.checked) document.getElementById('grammarScrub').checked = false;
});
document.getElementById('grammarScrub').addEventListener('change', (e) => {
  if (e.target.checked) document.getElementById('aggressiveScrub').checked = false;
});

// Ollama Settings Toggle
document.getElementById('ollamaMode').addEventListener('change', (e) => {
  document.getElementById('ollamaSettings').style.display = e.target.checked ? 'flex' : 'none';
});

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

// Listen for messages from context menu
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'scrubText') {
    const inputEl = document.getElementById('inputText');
    inputEl.value = request.text;
    
    // Auto-select deep scrub if nothing is selected
    if (!document.getElementById('aggressiveScrub').checked && !document.getElementById('grammarScrub').checked) {
      document.getElementById('aggressiveScrub').checked = true;
    }
    
    document.getElementById('scrubBtn').click();
  }
});
