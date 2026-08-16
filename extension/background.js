// Path to the offscreen document
const OFFSCREEN_DOCUMENT_PATH = '/offscreen.html';

// Helper to ensure the offscreen document exists
async function setupOffscreenDocument(path) {
  // Check if we already have an offscreen document
  const existingContexts = await chrome.runtime.getContexts({
    contextTypes: ['OFFSCREEN_DOCUMENT'],
    documentUrls: [chrome.runtime.getURL(path)]
  });

  if (existingContexts.length > 0) {
    return;
  }

  // Create document
  await chrome.offscreen.createDocument({
    url: path,
    reasons: ['DOM_PARSER'], // 'WORKERS' is invalid, use 'DOM_PARSER' as a general fallback for running JS
    justification: 'Run WebGPU WebLLM engine for AI text rewriting'
  });
}

// Handle messages from popup.js
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'paraphrase') {
    (async () => {
      try {
        await setupOffscreenDocument(OFFSCREEN_DOCUMENT_PATH);
        
        // Forward the message to the offscreen document
        chrome.runtime.sendMessage({
          action: 'paraphrase_offscreen',
          text: request.text
        }, (response) => {
          if (chrome.runtime.lastError) {
            console.error("Offscreen error:", chrome.runtime.lastError);
            sendResponse({ success: false, error: chrome.runtime.lastError.message });
          } else {
            // Forward the offscreen response back to popup
            sendResponse(response);
          }
        });
      } catch (err) {
        console.error("Setup offscreen error:", err);
        sendResponse({ success: false, error: err.toString() });
      }
    })();
    
    return true; // Keep channel open
  }
});
