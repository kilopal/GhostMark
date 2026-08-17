chrome.sidePanel
  .setPanelBehavior({ openPanelOnActionClick: true })
  .catch((error) => console.error(error));

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "ghostmark-scrub",
    title: "Scrub with GhostMark",
    contexts: ["selection"]
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "ghostmark-scrub") {
    // Open the side panel for the current window
    chrome.sidePanel.open({ windowId: tab.windowId });
    
    // Give the panel a moment to initialize if it wasn't open, then send the text
    setTimeout(() => {
      chrome.runtime.sendMessage({
        action: "scrubText",
        text: info.selectionText
      });
    }, 800);
  }
});
