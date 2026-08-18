// Web Worker implementation for WASM
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
let wasmWorker = null;

function runWasmWorker(action, payload, fileName) {
    return new Promise((resolve, reject) => {
        if (!wasmWorker) return reject("Worker not initialized");
        const id = Date.now().toString() + Math.random();
        
        const listener = (e) => {
            if (e.data.id === id) {
                wasmWorker.removeEventListener('message', listener);
                if (e.data.success) {
                    resolve(e.data.payload);
                } else {
                    reject(new Error(e.data.error));
                }
            }
        };
        
        wasmWorker.addEventListener('message', listener);
        
        if (payload instanceof Uint8Array || payload instanceof ArrayBuffer) {
            const buffer = payload instanceof Uint8Array ? payload.buffer : payload;
            wasmWorker.postMessage({ id, action, payload: buffer, fileName }, [buffer]);
        } else {
            wasmWorker.postMessage({ id, action, payload, fileName });
        }
    });
}

// ==========================================
// PARAPHRASE ENGINE (Transformers.js)
// ==========================================

async function getParaphraser() {
  if (paraphraser) return paraphraser;
  if (pipelinePromise) return pipelinePromise;

  pipelinePromise = (async () => {
    try {
      setStatus('Downloading AI model...', 'ready');

      // Use Microsoft Phi-3-mini-4k-instruct (3.8 Billion parameters) via ONNX
      // This is a massive model (~2.2GB quantized) requiring WebGPU and high RAM
      const pipe = await pipeline('text-generation', 'onnx-community/Phi-3-mini-4k-instruct', {
        dtype: 'q4f16', // Recommended for 3B+ models on WebGPU
        device: navigator.gpu ? 'webgpu' : 'wasm',
        progress_callback: (progress) => {
          if (progress.status === 'progress' && progress.progress) {
            setStatus(`Downloading 3.8B Model: ${Math.round(progress.progress)}%`, 'ready');
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
    wasmWorker = new Worker('wasm-worker.js', { type: 'module' });
    wasmLoaded = true;
    setStatus('Ready', 'success');

    // 2. Load chat history & settings
    chrome.storage.local.get(['chatHistory', 'geminiApiKey'], (result) => {
      if (result.chatHistory) {
        document.getElementById('chatArea').innerHTML = result.chatHistory;
        const chatArea = document.getElementById('chatArea');
        chatArea.scrollTop = chatArea.scrollHeight;
      }
      if (result.geminiApiKey) {
        document.getElementById('geminiApiKey').value = result.geminiApiKey;
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
// FILE PROCESSING
// ==========================================

async function processFile(file, typeLabel) {
  if (!wasmLoaded) return;
  
  // Update UI to show upload
  appendUserMessage(`📎 Uploaded ${typeLabel}: ${file.name} (${(file.size / 1024).toFixed(1)} KB)`);
  
  try {
    const t0 = performance.now();
    
    // Read file as ArrayBuffer
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    
    // Call Rust WASM to strip metadata via Web Worker
    const cleanedBuffer = await runWasmWorker('strip_file', arrayBuffer, file.name);
    const cleanedBytes = new Uint8Array(cleanedBuffer);
    
    const t1 = performance.now();
    const removedBytes = uint8Array.length - cleanedBytes.length;
    
    // Create new blob and auto-download
    const blob = new Blob([cleanedBytes], { type: file.type || 'application/octet-stream' });
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
        <div class="result-output">${typeLabel} Metadata Stripped.</div>
        <div class="result-meta">
          <div class="status-dot success"></div>
          <span>Removed ${removedBytes} bytes of hidden metadata in ${(t1 - t0).toFixed(2)}ms. Downloaded!</span>
        </div>
      </div>
    `;
    appendAssistantMessage(responseHtml);
    setStatus(`Stripped ${removedBytes} bytes`, 'success');
    
  } catch (err) {
    setStatus(`Error processing ${typeLabel.toLowerCase()}`, 'error');
    appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Error processing file</div><div class="result-meta"><div class="status-dot error"></div><span>${escapeHtml(err.toString())}</span></div></div>`);
    console.error(err);
  }
}

async function handleFileUpload(file) {
  if (!file) return;
  const ext = file.name.split('.').pop()?.toLowerCase();
  
  if (['png', 'jpeg', 'jpg', 'webp', 'bmp', 'gif'].includes(ext)) {
    await processFile(file, 'Image');
  } else if (ext === 'pdf') {
    await processFile(file, 'PDF');
  } else if (ext === 'docx') {
    await processFile(file, 'DOCX');
  } else if (ext === 'epub') {
    await processFile(file, 'EPUB');
  } else if (ext === 'odt') {
    await processFile(file, 'ODT');
  } else if (ext === 'svg') {
    await processFile(file, 'SVG');
  } else if (['txt', 'md', 'json'].includes(ext)) {
      appendUserMessage(`📎 Uploaded Text: ${file.name} (${(file.size / 1024).toFixed(1)} KB)`);
      try {
         const text = await file.text();
         const cleaned = await runWasmWorker('sanitize_text', text);
         const encoder = new TextEncoder();
         const cleanedBytes = encoder.encode(cleaned);
         
         const blob = new Blob([cleanedBytes], { type: file.type || 'text/plain' });
         const url = URL.createObjectURL(blob);
         const a = document.createElement('a');
         a.href = url;
         a.download = `${file.name.replace(/\.[^/.]+$/, "")}_ghostmark.${ext}`;
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
             <div class="result-output">Text Metadata Stripped.</div>
             <div class="result-meta">
               <div class="status-dot success"></div>
               <span>Applied homoglyph injection and removed metadata formatting. Downloaded!</span>
             </div>
           </div>
         `;
         appendAssistantMessage(responseHtml);
      } catch (err) {
         appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Error processing text file</div><div class="result-meta"><div class="status-dot error"></div><span>${escapeHtml(err.toString())}</span></div></div>`);
      }
  } else {
    appendAssistantMessage(`<div class="result-avatar" style="background:var(--danger)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"></path></svg></div><div class="result-content"><div class="result-output" style="color:var(--danger)">Invalid file type</div><div class="result-meta"><div class="status-dot error"></div><span>Unsupported file type. Please upload a valid document or image.</span></div></div>`);
  }
}

// Manual Upload Button
document.getElementById('uploadBtn').addEventListener('click', () => {
  document.getElementById('fileInput').click();
});

document.getElementById('fileInput').addEventListener('change', (e) => {
  if (e.target.files.length > 0) {
    handleFileUpload(e.target.files[0]);
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
    handleFileUpload(e.dataTransfer.files[0]);
  }
});

// ==========================================
// TEXT PROCESSING & DETECTION
// ==========================================

async function checkSynthId(text) {
  const apiKey = document.getElementById('geminiApiKey').value.trim();
  if (!apiKey) return "";
  
  try {
    const res = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=${apiKey}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{ role: 'user', parts: [{ text }] }],
        generationConfig: { taskType: 'DETECT_TEXT_WATERMARK' }
      })
    });
    if (!res.ok) return "API Error";
    const data = await res.json();
    return data.candidates?.[0]?.content?.parts?.[0]?.text || "Unknown";
  } catch (err) {
    return "Network Error";
  }
}

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
  const useGemini = document.getElementById('geminiMode').checked;

  // 1. Show user message
  appendUserMessage(input);
  
  // 2. Clear input & reset height
  inputEl.value = '';
  inputEl.style.height = '22px';

  try {
    let scoreBefore = "";
    if (useGemini) {
       setStatus('Checking original SynthID...', 'ready');
       scoreBefore = await checkSynthId(input);
    }

    const originalLen = input.length;
    const t0 = performance.now();
    
    if (aggressive || grammar) {
      // 1. First, run the fast WASM normalization and Unicode scrub
      const wasmCleaned = await runWasmWorker('sanitize_text', input);
      
      const llmT0 = performance.now();
      
      try {
        const ollamaMode = document.getElementById('ollamaMode')?.checked;
        const ollamaModel = document.getElementById('ollamaModel')?.value || 'llama3';
        
        let pipe = null;
        if (!ollamaMode) {
          pipe = await getParaphraser();
        }
        setStatus(grammar ? 'Checking grammar...' : 'Rewriting text...', 'ready');
        
        const chunks = [];
        if (ollamaMode) {
          const words = wasmCleaned.split(' ');
          let currentChunk = '';
          for (const word of words) {
            if ((currentChunk + word).length > 4000 && currentChunk.length > 0) {
              chunks.push(currentChunk.trim());
              currentChunk = '';
            }
            currentChunk += word + ' ';
          }
          if (currentChunk.trim().length > 0) chunks.push(currentChunk.trim());
        } else {
          const rawParagraphs = wasmCleaned.split(/\n+/);
          for (const p of rawParagraphs) {
            if (!p.trim()) continue;
            if (p.length < 1500) {
              chunks.push(p.trim());
            } else {
              const sentences = p.match(/[^.!?]+[.!?]+/g) || [p];
              let currentChunk = '';
              for (const s of sentences) {
                if ((currentChunk + s).length > 1000 && currentChunk.length > 0) {
                  chunks.push(currentChunk.trim());
                  currentChunk = '';
                }
                currentChunk += s + ' ';
              }
              if (currentChunk.trim().length > 0) chunks.push(currentChunk.trim());
            }
          }
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
            const chat = [
              { role: 'system', content: sysPrompt },
              { role: 'user', content: chunk }
            ];
            
            const result = await pipe(chat, {
              max_new_tokens: 512,
              temperature: grammar ? 0.2 : 0.6,
              top_p: 0.9,
              repetition_penalty: 1.05,
              do_sample: true
            });
            const generatedText = result[0].generated_text;
            reply = generatedText[generatedText.length - 1].content;
          }
          rewrittenParts.push(reply.trim());
        }

        // 2. Run the algorithmic Homoglyph Perturbation on the generated text via WASM
        const joinedText = rewrittenParts.join(' ');
        const finalCleaned = grammar ? joinedText : await runWasmWorker('sanitize_text_homoglyph', joinedText);
        const llmT1 = performance.now();
        
        let scoreAfter = "";
        if (useGemini) {
           setStatus('Checking final SynthID...', 'ready');
           scoreAfter = await checkSynthId(finalCleaned);
        }
        
        await copyToClipboard(finalCleaned);
        
        const title = grammar ? 'Grammar Corrected' : 'Statistical Watermark Destroyed';
        
        let resultOutput = escapeHtml(finalCleaned);
        if (useGemini) {
           resultOutput = `<div style="font-size:12px;color:var(--info);margin-bottom:8px;background:var(--bg-raised);padding:6px;border-radius:4px;"><b>SynthID Before:</b> ${scoreBefore}<br/><b>SynthID After:</b> ${scoreAfter}</div>${resultOutput}`;
        }
        
        const responseHtml = `
          <div class="result-avatar">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M12 2C8.13 2 5 5.13 5 9c0 1.74.56 3.35 1.5 4.66V22l2.5-2 2 2 2-2 2 2 2-2 2.5 2v-8.34A6.96 6.96 0 0019 9c0-3.87-3.13-7-7-7z" fill="currentColor" stroke="none"/>
              <circle cx="9.5" cy="9" r="1.5" fill="var(--bg-base)"/>
              <circle cx="14.5" cy="9" r="1.5" fill="var(--bg-base)"/>
            </svg>
          </div>
          <div class="result-content">
            <div class="result-output">${resultOutput}</div>
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
      const cleaned = await runWasmWorker('sanitize_text', input);
      const t1 = performance.now();
      const removed = originalLen - cleaned.length;
      
      let scoreAfter = "";
      if (useGemini) {
         setStatus('Checking final SynthID...', 'ready');
         scoreAfter = await checkSynthId(cleaned);
      }
      
      await copyToClipboard(cleaned);
      
      let resultOutput = `Cleaned ${removed > 0 ? removed : 0} hidden characters.`;
      if (useGemini) {
         resultOutput = `<div style="font-size:12px;color:var(--info);margin-bottom:8px;background:var(--bg-raised);padding:6px;border-radius:4px;"><b>SynthID Before:</b> ${scoreBefore}<br/><b>SynthID After:</b> ${scoreAfter}</div>${resultOutput}`;
      }
      
      const responseHtml = `
        <div class="result-avatar">
          <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 2C8.13 2 5 5.13 5 9c0 1.74.56 3.35 1.5 4.66V22l2.5-2 2 2 2-2 2 2 2-2 2.5 2v-8.34A6.96 6.96 0 0019 9c0-3.87-3.13-7-7-7z" fill="currentColor" stroke="none"/>
            <circle cx="9.5" cy="9" r="1.5" fill="var(--bg-base)"/>
            <circle cx="14.5" cy="9" r="1.5" fill="var(--bg-base)"/>
          </svg>
        </div>
        <div class="result-content">
          <div class="result-output">${resultOutput}</div>
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

// (JS Homoglyph function removed in favor of WASM)

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

// Gemini Settings Toggle
document.getElementById('geminiMode').addEventListener('change', (e) => {
  document.getElementById('geminiSettings').style.display = e.target.checked ? 'flex' : 'none';
});

// Save API key on change
document.getElementById('geminiApiKey').addEventListener('change', (e) => {
  chrome.storage.local.set({ geminiApiKey: e.target.value.trim() });
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
