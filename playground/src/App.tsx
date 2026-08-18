import React, { useState, useEffect, useRef } from 'react';
import { ArrowUp, Paperclip, Download, FileCode, Globe, Terminal, AlertCircle, X, ChevronDown, CheckCircle2, Code2, Trash2, Menu, Bot } from 'lucide-react';
import initWasm, { sanitize_text_wasm, strip_image_bytes_wasm, strip_pdf_metadata_wasm, strip_docx_metadata_wasm, strip_epub_metadata_wasm, strip_odt_metadata_wasm, strip_svg_metadata_wasm } from './pkg/ghostmark_wasm.js';
import wasmUrl from './pkg/ghostmark_wasm_bg.wasm?url';
import { pipeline, env } from '@huggingface/transformers';

type Message = {
  role: 'user' | 'assistant';
  content: string;
  isScrubbing?: boolean;
  isDownloadable?: boolean;
  fileName?: string;
  fileBytes?: Uint8Array;
  fileType?: string;
};

type Session = {
  id: string;
  title: string;
  messages: Message[];
  updatedAt: number;
};

export default function App() {
  const [inputText, setInputText] = useState('');
  
  const [sessions, setSessions] = useState<Session[]>(() => {
    const saved = localStorage.getItem('ghostmark-sessions');
    if (saved) {
      try { return JSON.parse(saved); } catch(e) {}
    }
    return [];
  });
  const [activeSessionId, setActiveSessionId] = useState<string | null>(() => {
    return localStorage.getItem('ghostmark-active-session');
  });

  useEffect(() => {
    const savable = sessions.map(s => ({
       ...s,
       messages: s.messages.map((m: Message) => ({ ...m, fileBytes: undefined }))
    }));
    localStorage.setItem('ghostmark-sessions', JSON.stringify(savable));
  }, [sessions]);

  useEffect(() => {
    if (activeSessionId) {
      localStorage.setItem('ghostmark-active-session', activeSessionId);
    }
  }, [activeSessionId]);

  const activeSession = sessions.find(s => s.id === activeSessionId);
  const messages = activeSession ? activeSession.messages : [];

  const updateMessages = (updater: any) => {
    setSessions(prev => {
      let currentId = activeSessionId;
      let currentSession = prev.find(s => s.id === currentId);
      
      if (!currentSession) {
         currentId = Date.now().toString();
         currentSession = { id: currentId, title: 'New Chat', messages: [], updatedAt: Date.now() };
      }

      const updatedMessages = typeof updater === 'function' ? updater(currentSession.messages) : updater;
      
      let newTitle = currentSession.title;
      if (currentSession.messages.length === 0 && updatedMessages.length > 0 && updatedMessages[0].role === 'user') {
         newTitle = updatedMessages[0].content.slice(0, 30) + (updatedMessages[0].content.length > 30 ? '...' : '');
      }

      const newSession = { ...currentSession, messages: updatedMessages, title: newTitle, updatedAt: Date.now() };
      
      if (!prev.find(s => s.id === currentId)) {
          return [newSession, ...prev];
      }
      
      return prev.map(s => s.id === currentId ? newSession : s).sort((a, b) => b.updatedAt - a.updatedAt);
    });
    
    setSessions(s => {
       if (s.length > 0 && !activeSessionId) {
           setTimeout(() => setActiveSessionId(s[0].id), 0);
       }
       return s;
    });
  };

  const deleteSession = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setSessions(prev => {
      const next = prev.filter(s => s.id !== id);
      if (activeSessionId === id) {
        setTimeout(() => setActiveSessionId(next.length > 0 ? next[0].id : null), 0);
      }
      return next;
    });
  };

  const [isProcessing, setIsProcessing] = useState(false);
  const [processingTime, setProcessingTime] = useState(0);
  const [wasmEngine, setWasmEngine] = useState<any>(null);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  
  // Settings State
  const [showSettings, setShowSettings] = useState(false);
  const [useHomoglyphs, setUseHomoglyphs] = useState(true);
  const [llmMode, setLlmMode] = useState<'none' | 'groq' | 'ollama' | 'webgpu'>('none');
  const [groqKey, setGroqKey] = useState('');
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434');
  const [ollamaModel, setOllamaModel] = useState('llama3');

  // Transformers.js State
  const [hfPipeline, setHfPipeline] = useState<any>(null);
  const [hfProgress, setHfProgress] = useState(0);

  // File Upload State
  const [isHoveringFile, setIsHoveringFile] = useState(false);
  
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const feedEndRef = useRef<HTMLDivElement>(null);

  // Auto-resize textarea
  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.style.height = 'auto';
      textAreaRef.current.style.height = Math.min(textAreaRef.current.scrollHeight, 200) + 'px';
    }
  }, [inputText]);

  // Scroll to bottom of feed
  useEffect(() => {
    feedEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isProcessing]);

  // Processing Timer
  useEffect(() => {
    let interval: any;
    if (isProcessing) {
      setProcessingTime(0);
      interval = setInterval(() => {
        setProcessingTime(prev => prev + 0.1);
      }, 100);
    } else {
      setProcessingTime(0);
    }
    return () => clearInterval(interval);
  }, [isProcessing]);

  // Load WASM on mount
  useEffect(() => {
    async function loadWasm() {
      try {
        await initWasm(wasmUrl);
        setWasmEngine(() => sanitize_text_wasm);
      } catch (err) {
        console.error("Failed to load WASM:", err);
      }
    }
    loadWasm();
  }, []);

  const getParaphraser = async () => {
    if (hfPipeline) return hfPipeline;
    env.allowLocalModels = false;
    env.backends.onnx.wasm!.numThreads = 1;
    env.backends.onnx.wasm!.proxy = false;

    const pipe = await pipeline('text-generation', 'onnx-community/Llama-3.2-1B-Instruct', {
      dtype: 'q8',
      device: (navigator as any).gpu ? 'webgpu' : 'wasm',
      progress_callback: (progress: any) => {
        if (progress.status === 'progress' && progress.progress) {
          setHfProgress(Math.round(progress.progress));
        }
      }
    });
    setHfPipeline(() => pipe);
    return pipe;
  };

  const handleProcessText = async () => {
    if (!inputText.trim() || isProcessing) return;
    
    const textToProcess = inputText;
    setInputText('');
    if (textAreaRef.current) textAreaRef.current.style.height = 'auto';

    updateMessages((prev: Message[]) => [...prev, { role: 'user', content: textToProcess }]);
    setIsProcessing(true);

    try {
      let currentText = textToProcess;
      
      // 1. Pre-processing
      if (wasmEngine) {
        currentText = wasmEngine(textToProcess, useHomoglyphs && llmMode === 'none');
      }

      // 2. LLM Engine
      if (llmMode === 'groq' && groqKey) {
         const res = await fetch('https://api.groq.com/openai/v1/chat/completions', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${groqKey}` },
            body: JSON.stringify({
              model: 'llama3-70b-8192',
              messages: [
                { role: 'system', content: 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. Preserve the exact same meaning, facts, and names. Output ONLY the rewritten text, nothing else.' },
                { role: 'user', content: currentText }
              ],
              temperature: 0.6,
            })
         });
         if (!res.ok) throw new Error("Groq API Error.");
         const data = await res.json();
         currentText = data.choices[0].message.content;
      } else if (llmMode === 'ollama') {
         const res = await fetch(`${ollamaUrl}/api/generate`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              model: ollamaModel,
              system: 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. Preserve the exact same meaning. Output ONLY the rewritten text.',
              prompt: currentText,
              stream: false
            })
         });
         if (!res.ok) throw new Error("Ollama server not responding.");
         const data = await res.json();
         currentText = data.response;
      } else if (llmMode === 'webgpu') {
         const pipe = await getParaphraser();
         
         // Split into ~400 char chunks. The 1B model is too small to handle
         // full essays in one shot — it hallucinates. Chunking is what made
         // the extension achieve 0% AI detection consistently.
         const chunks: string[] = [];
         for (let i = 0; i < currentText.length; i += 400) {
           chunks.push(currentText.slice(i, i + 400));
         }

         const rewrittenParts: string[] = [];
         for (const chunk of chunks) {
           const chat = [
             { role: 'system', content: 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. You MUST preserve the exact same meaning, names, genders, and pronouns (he/she/they) as the original. Output only the rewritten text.' },
             { role: 'user', content: chunk }
           ];
           const result = await pipe(chat, {
             max_new_tokens: 2048,
             temperature: 0.6,
             top_p: 0.9,
             repetition_penalty: 1.05,
             do_sample: true
           });
           const out = result[0].generated_text;
           rewrittenParts.push(out[out.length - 1].content.trim());
         }
         currentText = rewrittenParts.join(' ');
      }

      // 3. Post-processing
      if (useHomoglyphs && llmMode !== 'none' && wasmEngine) {
         currentText = wasmEngine(currentText, true);
      }

      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: currentText }]);
    } catch (err: any) {
      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Error: ${err.message}` }]);
    } finally {
      setIsProcessing(false);
    }
  };

  const processFile = async (file: File, wasmFunc: any) => {
    if (!wasmEngine) return;
    updateMessages((prev: Message[]) => [...prev, { role: 'user', content: `Attached File: ${file.name}` }]);
    setIsProcessing(true);
    
    try {
      const arrayBuffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);
      const cleanedBytes = wasmFunc(uint8Array);
      const removedBytes = uint8Array.length - cleanedBytes.length;
      
      updateMessages((prev: Message[]) => [...prev, { 
        role: 'assistant', 
        content: `Scrubbed successfully! Removed ${removedBytes} bytes of hidden metadata/tracking data.`,
        isDownloadable: true,
        fileName: file.name,
        fileBytes: cleanedBytes,
        fileType: file.type
      }]);
    } catch (err: any) {
      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Error processing file: ${err.message}` }]);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleFileUpload = async (file: File) => {
    if (!file) return;
    const ext = file.name.split('.').pop()?.toLowerCase();
    
    if (['png', 'jpeg', 'jpg', 'webp', 'bmp', 'gif'].includes(ext || '')) {
      await processFile(file, strip_image_bytes_wasm);
    } else if (ext === 'pdf') {
      await processFile(file, strip_pdf_metadata_wasm);
    } else if (ext === 'docx') {
      await processFile(file, strip_docx_metadata_wasm);
    } else if (ext === 'epub') {
      await processFile(file, strip_epub_metadata_wasm);
    } else if (ext === 'odt') {
      await processFile(file, strip_odt_metadata_wasm);
    } else if (ext === 'svg') {
      await processFile(file, strip_svg_metadata_wasm);
    } else if (['txt', 'md', 'json'].includes(ext || '')) {
      // For text files, read as string, run WASM, then download as Blob
      updateMessages((prev: Message[]) => [...prev, { role: 'user', content: `Attached File: ${file.name}` }]);
      try {
         setIsProcessing(true);
         const text = await file.text();
         const cleaned = sanitize_text_wasm(text, false);
         const encoder = new TextEncoder();
         const cleanedBytes = encoder.encode(cleaned);
         
         updateMessages((prev: Message[]) => [...prev, { 
            role: 'assistant', 
            content: `Scrubbed successfully! Applied homoglyph injection and removed metadata formatting.`,
            isDownloadable: true,
            fileName: file.name,
            fileBytes: cleanedBytes,
            fileType: file.type || 'text/plain'
         }]);
      } catch (err: any) {
         updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Error processing text file: ${err.message}` }]);
      } finally {
         setIsProcessing(false);
      }
    } else {
      updateMessages((prev: Message[]) => [...prev, { role: 'user', content: `Attached File: ${file.name}` }]);
      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: 'Error: Unsupported file type. GhostMark Web supports Text, Images, PDF, DOCX, EPUB, ODT, and SVG.' }]);
    }
  };

  const downloadFile = (bytes: Uint8Array, originalName: string, type: string) => {
    const blob = new Blob([bytes as any], { type });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const nameParts = originalName.split('.');
    const ext = nameParts.pop();
    a.download = `${nameParts.join('.')}_ghostmark.${ext}`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };

  const copyToClipboard = (text: string, e: React.MouseEvent<HTMLButtonElement>) => {
    navigator.clipboard.writeText(text);
    const btn = e.currentTarget;
    const originalHTML = btn.innerHTML;
    btn.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    setTimeout(() => { btn.innerHTML = originalHTML; }, 2000);
  };

  // Helper for rendering the engine dropdown name
  const getEngineName = () => {
    switch (llmMode) {
      case 'none': return 'WASM Only';
      case 'groq': return 'BYOK (Groq)';
      case 'ollama': return 'Local Ollama';
      case 'webgpu': return 'Local WebGPU 1B';
    }
  };

  return (
    <div className="layout"
         onDragOver={(e) => { e.preventDefault(); setIsHoveringFile(true); }}
         onDragLeave={() => setIsHoveringFile(false)}
         onDrop={(e) => {
           e.preventDefault();
           setIsHoveringFile(false);
           if (e.dataTransfer.files && e.dataTransfer.files[0]) {
             handleFileUpload(e.dataTransfer.files[0]);
           }
         }}>
      {/* Mobile Sidebar Overlay */}
      {isSidebarOpen && <div className="sidebar-overlay" onClick={() => setIsSidebarOpen(false)} />}
      
      <aside className={`sidebar ${isSidebarOpen ? 'open' : ''}`}>
        <button className="sidebar-btn sidebar-new-chat" onClick={() => setActiveSessionId(Date.now().toString())}>
          <span>New Chat</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
        </button>

        <div className="sidebar-links">
          <a href="https://github.com/kilopal/GhostMark/tree/main/cli" target="_blank" rel="noreferrer" className="sidebar-btn" style={{textDecoration: 'none'}}>
             <Terminal size={16} />
             <span>GhostMark CLI</span>
          </a>
          <a href="https://github.com/kilopal/GhostMark/tree/main/wasm" target="_blank" rel="noreferrer" className="sidebar-btn" style={{textDecoration: 'none'}}>
             <Code2 size={16} />
             <span>WASM API</span>
          </a>
          <a href="https://github.com/kilopal/GhostMark/tree/main/extension" target="_blank" rel="noreferrer" className="sidebar-btn" style={{textDecoration: 'none'}}>
             <Globe size={16} />
             <span>Chrome Extension</span>
          </a>
        </div>

        <div className="sidebar-section-title">Recent Chats</div>

        <div className="sidebar-sessions">
          {sessions.length === 0 && (
             <div className="sidebar-empty">No recent chats</div>
          )}
          {sessions.map(s => (
            <button 
              key={s.id} 
              className={`sidebar-btn ${s.id === activeSessionId ? 'active' : ''}`} 
              onClick={() => setActiveSessionId(s.id)}
            >
              <div className="session-title">{s.title || 'New Chat'}</div>
              <div 
                className="delete-session-btn" 
                onClick={(e) => deleteSession(s.id, e)}
                title="Delete Chat"
              >
                <Trash2 size={14} />
              </div>
            </button>
          ))}
        </div>
        
        <div className="sidebar-footer">
           <button className="sidebar-btn" onClick={() => setShowSettings(true)}>
              <div className="avatar user" style={{ width: 24, height: 24, fontSize: '0.75rem', borderRadius: '50%', background: '#fff', color: '#000', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>G</div>
              <span>Engine Settings</span>
           </button>
        </div>
      </aside>

      <div className="app-content">

      {/* Drag Overlay */}
      {isHoveringFile && (
        <div className="drag-overlay animate-fade-in">
          <div className="drag-overlay-content">
            <FileCode size={48} color="var(--text-primary)" style={{ marginBottom: '1rem' }} />
            <h2 style={{ fontSize: '1.5rem', fontWeight: 600 }}>Drop file to scrub metadata</h2>
            <p style={{ color: 'var(--text-secondary)', marginTop: '0.5rem' }}>PDF, DOCX, EPUB, SVG, ODT, PNG, JPEG</p>
          </div>
        </div>
      )}

      {/* Top Navigation */}
      <header className="top-nav">
        <div className="nav-brand">
          <button className="mobile-menu-btn" onClick={() => setIsSidebarOpen(true)}>
             <Menu size={20} />
          </button>
          <div className="brand-logo">👻</div>
          <span style={{ fontWeight: 600, fontSize: '1.1rem' }} className="brand-title">GhostMark</span>
        </div>
        
        <div className="nav-actions">
          {/* OSS Badges */}
          <a href="https://github.com/kilopal/GhostMark" target="_blank" rel="noreferrer" className="oss-btn">
            <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round"><path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"></path><path d="M9 18c-4.51 2-5-2-7-2"></path></svg>
            <span style={{ fontWeight: 500 }}>Star on GitHub</span>
          </a>

          {/* Engine Selector Dropdown */}
          <div className="dropdown">
            <button className="dropdown-btn" onClick={() => setShowSettings(!showSettings)}>
              <span style={{ color: 'var(--text-secondary)' }}>Engine:</span>
              <span style={{ fontWeight: 500 }}>{getEngineName()}</span>
              <ChevronDown size={16} color="var(--text-secondary)" />
            </button>
            
            {showSettings && (
              <div className="settings-modal animate-fade-in">
                <div className="settings-header">
                  <h3>Engine Settings</h3>
                  <button onClick={() => setShowSettings(false)} className="close-btn"><X size={18} /></button>
                </div>
                
                <div className="settings-body">
                  <div className="setting-group">
                    <label className="checkbox-label">
                      <input type="checkbox" checked={useHomoglyphs} onChange={(e) => setUseHomoglyphs(e.target.checked)} />
                      <span>Homoglyph Injection (Layer B)</span>
                    </label>
                    <p className="setting-desc">Injects zero-width characters to bypass statistical AI detectors.</p>
                  </div>

                  <div className="setting-group">
                    <label className="select-label">Deep Scrub Engine</label>
                    <div className="engine-grid">
                      <button className={`engine-btn ${llmMode === 'none' ? 'active' : ''}`} onClick={() => setLlmMode('none')}>WASM Only</button>
                      <button className={`engine-btn ${llmMode === 'groq' ? 'active' : ''}`} onClick={() => setLlmMode('groq')}>BYOK (Groq)</button>
                      <button className={`engine-btn ${llmMode === 'ollama' ? 'active' : ''}`} onClick={() => setLlmMode('ollama')}>Ollama (10B)</button>
                      <button className={`engine-btn ${llmMode === 'webgpu' ? 'active' : ''}`} onClick={() => {
                        if (window.innerWidth < 768 || /Mobi|Android/i.test(navigator.userAgent)) {
                          alert("WebGPU 1B models require 2GB+ of free RAM and may crash mobile browsers. Please use a desktop device or another engine.");
                        } else {
                          setLlmMode('webgpu');
                        }
                      }}>WebGPU (1B)</button>
                    </div>
                  </div>

                  {llmMode === 'groq' && (
                    <div className="setting-group animate-fade-in">
                      <label className="select-label">Groq API Key</label>
                      <input type="password" value={groqKey} onChange={(e) => setGroqKey(e.target.value)} placeholder="gsk_..." className="modern-input" />
                    </div>
                  )}

                  {llmMode === 'ollama' && (
                    <div className="setting-group animate-fade-in">
                      <label className="select-label">Ollama Configuration</label>
                      <input type="text" value={ollamaUrl} onChange={(e) => setOllamaUrl(e.target.value)} placeholder="http://localhost:11434" className="modern-input mb-2" />
                      <input type="text" value={ollamaModel} onChange={(e) => setOllamaModel(e.target.value)} placeholder="llama3" className="modern-input" />
                    </div>
                  )}

                  {llmMode === 'webgpu' && (
                    <div className="warning-box animate-fade-in">
                      <AlertCircle size={18} color="var(--text-primary)" style={{ flexShrink: 0 }} />
                      <p><strong>Heads up:</strong> WebGPU will download a ~1GB Llama model into your browser cache on first run. Requires a modern GPU.</p>
                    </div>
                  )}
                  
                  <div className="privacy-badge">
                    <CheckCircle2 size={14} color="var(--success)" />
                    <span>100% Local Browser Execution. No data is sent to our servers.</span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <div className="main-content">
        
        {/* Chat Feed */}
        <div className="chat-feed">
          {messages.length === 0 ? (
            <div className="empty-state">
              <div className="empty-logo">👻</div>
              <h2>How can I scrub your data today?</h2>
              <p className="app-description">
                GhostMark 👻 A blazing-fast, memory-safe tool written in Rust to strip Anthropic, OpenAI, and EU-mandated AI watermarks (C2PA &amp; Unicode) from text and images.
              </p>
              
              <div className="suggested-actions">
                 <button onClick={() => fileInputRef.current?.click()} className="suggest-btn">
                   <FileCode size={18} />
                   Scrub a PDF or DOCX
                 </button>
                 <a href="https://github.com/kilopal/GhostMark/tree/main/extension" target="_blank" rel="noreferrer" className="suggest-btn">
                   <Globe size={18} />
                   Get Chrome Extension
                 </a>
                 <a href="https://github.com/kilopal/GhostMark/tree/main/cli" target="_blank" rel="noreferrer" className="suggest-btn">
                   <Terminal size={18} />
                   Use the Rust CLI
                 </a>
                 <a href="https://github.com/kilopal/GhostMark/tree/main/wasm" target="_blank" rel="noreferrer" className="suggest-btn">
                   <Code2 size={18} />
                   Developer WASM API
                 </a>
                 <a href="https://github.com/kilopal/GhostMark/tree/main/skills/ghostmark-clean" target="_blank" rel="noreferrer" className="suggest-btn">
                   <Bot size={18} />
                   Add AI Agent Skill
                 </a>
              </div>
            </div>
          ) : (
            messages.map((msg, idx) => (
              <div key={idx} className={`message-row ${msg.role}`}>
                <div className="message-content animate-fade-in">
                  
                  {/* Avatar */}
                  {msg.role === 'user' ? (
                     <div className="avatar user">You</div>
                  ) : (
                     <div className="avatar assistant">👻</div>
                  )}

                  {/* Message Body */}
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div className="prose">{msg.content}</div>
                    
                    {/* Assistant Actions */}
                    {msg.role === 'assistant' && (
                      <div className="msg-actions">
                        {msg.isDownloadable && msg.fileBytes ? (
                          <button onClick={() => downloadFile(msg.fileBytes!, msg.fileName!, msg.fileType!)} className="action-btn primary">
                            <Download size={14} /> Download File
                          </button>
                        ) : (
                          <button onClick={(e) => copyToClipboard(msg.content, e)} className="action-btn icon-only" title="Copy Text">
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                          </button>
                        )}
                      </div>
                    )}
                  </div>

                </div>
              </div>
            ))
          )}

          {/* Loading Indicator */}
          {isProcessing && (
             <div className="message-row assistant">
               <div className="message-content animate-fade-in">
                 <div className="avatar assistant pulse-bg">👻</div>
                 <div style={{ paddingTop: '6px', color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
                   {llmMode === 'webgpu' && hfProgress > 0 && hfProgress < 100 
                      ? `Downloading Model... ${hfProgress}%` 
                      : <span className="animate-pulse">Processing... ({processingTime.toFixed(1)}s)</span>
                   }
                 </div>
               </div>
             </div>
          )}
          <div ref={feedEndRef} style={{ height: '120px' }} />
        </div>

        {/* Input Box Area */}
        <div className="input-container">
          <div className="input-wrapper">
            <input type="file" ref={fileInputRef} style={{ display: 'none' }} onChange={(e) => { if (e.target.files && e.target.files[0]) handleFileUpload(e.target.files[0]); }} />
            
            <button className="attach-btn" onClick={() => fileInputRef.current?.click()} title="Attach File">
              <Paperclip size={20} />
            </button>
            
            <textarea
              ref={textAreaRef}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleProcessText();
                }
              }}
              placeholder="Message GhostMark..."
              rows={1}
              disabled={isProcessing || !wasmEngine}
            />

            <button className="submit-btn" onClick={handleProcessText} disabled={!inputText.trim() || isProcessing || !wasmEngine}>
              <ArrowUp size={20} strokeWidth={2.5} />
            </button>
          </div>
          <div className="disclaimer">
            GhostMark WASM Engine handles PDF, DOCX, EPUB, ODT, SVG and Images directly in your browser.
          </div>
        </div>
      </div>

      </div>

      {/* Settings Backdrop */}
      {showSettings && <div className="modal-backdrop" onClick={() => setShowSettings(false)} />}
    </div>
  );
}
