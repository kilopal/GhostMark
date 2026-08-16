import { MLCEngine } from "@mlc-ai/web-llm";

// We'll use Phi-3-mini as it's very capable but smaller than Llama 3
const MODEL = "Phi-3-mini-4k-instruct-q4f16_1-MLC";
let engine = null;
let initPromise = null;

// Global error handlers to send errors back to popup
window.onerror = function(message, source, lineno, colno, error) {
  chrome.runtime.sendMessage({ action: 'progress', status: 'error', message: `Offscreen Error: ${message}` });
};
window.addEventListener('unhandledrejection', function(event) {
  chrome.runtime.sendMessage({ action: 'progress', status: 'error', message: `Offscreen Promise Error: ${event.reason}` });
});

async function initEngine() {
  if (engine) return engine;
  
  if (initPromise) {
    return initPromise; // Wait for the existing initialization to finish
  }

  initPromise = (async () => {
    try {
      chrome.runtime.sendMessage({ action: 'progress', status: 'downloading', message: `Step 1: Instantiating MLCEngine...` });
      const newEngine = new MLCEngine();
      
      chrome.runtime.sendMessage({ action: 'progress', status: 'downloading', message: `Step 2: Starting model reload (this may take a moment)...` });
      await newEngine.reload(MODEL, {
        initProgressCallback: (progress) => {
          chrome.runtime.sendMessage({
            action: 'progress',
            status: 'downloading',
            message: `Downloading AI model (1.8GB): ${Math.round(progress.progress * 100)}%`
          });
        }
      });
      
      chrome.runtime.sendMessage({ action: 'progress', status: 'downloading', message: `Step 3: Model loaded successfully.` });
      engine = newEngine;
      return engine;
    } catch (err) {
      initPromise = null; // Reset so they can try again
      chrome.runtime.sendMessage({ action: 'progress', status: 'error', message: `Init Error: ${err.toString()}` });
      throw err;
    }
  })();
  
  return initPromise;
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'paraphrase_offscreen') {
    (async () => {
      try {
        const eng = await initEngine();
        chrome.runtime.sendMessage({ 
          action: 'progress', 
          status: 'processing', 
          message: 'Neural network is rewriting text...' 
        });
        
        const prompt = `Rewrite the following text to sound casual, natural, and human. Do not summarize. Keep the same length and details. Output only the rewritten text. Do not include introductory or concluding remarks.\n\nText:\n${request.text}`;
        
        const reply = await eng.chat.completions.create({
          messages: [{ role: 'user', content: prompt }],
          temperature: 0.7,
        });
        
        sendResponse({ success: true, text: reply.choices[0].message.content });
      } catch (err) {
        console.error("WebLLM Error:", err);
        sendResponse({ success: false, error: err.toString() });
      }
    })();
    return true; // Keep the message channel open for async response
  }
});
