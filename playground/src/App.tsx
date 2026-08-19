import React, { useState, useEffect, useRef } from 'react';
import { ArrowUp, Download, FileCode, Globe, Terminal, X, ChevronDown, CheckCircle2, Code2, Trash2, Menu, Square, RotateCcw, Sun, Moon, Image, FileText } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

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
  const activeSessionIdRef = useRef<string | null>(activeSessionId);

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
      let currentId = activeSessionIdRef.current;
      let currentSession = prev.find(s => s.id === currentId);
      
      if (!currentSession) {
         currentId = currentId || Date.now().toString() + Math.random().toString().slice(2, 6);
         currentSession = { id: currentId as string, title: 'New Chat', messages: [], updatedAt: Date.now() };
         activeSessionIdRef.current = currentId;
      }

      const updatedMessages = typeof updater === 'function' ? updater(currentSession.messages) : updater;
      
      let newTitle = currentSession.title;
      if (currentSession.messages.length === 0 && updatedMessages.length > 0 && updatedMessages[0].role === 'user') {
         newTitle = updatedMessages[0].content.slice(0, 30) + (updatedMessages[0].content.length > 30 ? '...' : '');
      }

      const newSession = { ...currentSession, messages: updatedMessages, title: newTitle, updatedAt: Date.now() };
      
      if (!prev.find(s => s.id === currentId)) {
          setTimeout(() => setActiveSessionId(currentId), 0);
          return [newSession, ...prev];
      }
      
      return prev.map(s => s.id === currentId ? newSession : s).sort((a, b) => b.updatedAt - a.updatedAt);
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

  const [processingSessions, setProcessingSessions] = useState<Record<string, boolean>>({});
  const [abortControllers, setAbortControllers] = useState<Record<string, AbortController>>({});
  const [processingTime, setProcessingTime] = useState(0);
  const [wasmWorker, setWasmWorker] = useState<Worker | null>(null);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [theme, setTheme] = useState<'light' | 'dark'>((localStorage.getItem('ghostmark_theme') as any) || 'light');
  
  // Apply theme
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('ghostmark_theme', theme);
  }, [theme]);
  
  // Settings State
  const [showSettings, setShowSettings] = useState(false);
  const [useHomoglyphs, setUseHomoglyphs] = useState(true);
  const [useShatterSynthId, setUseShatterSynthId] = useState(false);
  const [useSynthIdDetect, setUseSynthIdDetect] = useState(false);
  const [llmMode, setLlmMode] = useState<'none' | 'cloud' | 'ollama' | 'nano'>('none');
  const [cloudProvider, setCloudProvider] = useState<'groq' | 'openai' | 'gemini' | 'deepseek'>('groq');
  
  const [groqKey, setGroqKey] = useState(localStorage.getItem('ghostmark_groq_key') || '');
  const [openaiKey, setOpenaiKey] = useState(localStorage.getItem('ghostmark_openai_key') || '');
  const [deepseekKey, setDeepseekKey] = useState(localStorage.getItem('ghostmark_deepseek_key') || '');
  const [geminiKey, setGeminiKey] = useState(localStorage.getItem('ghostmark_gemini_key') || '');
  
  const [ollamaUrl, setOllamaUrl] = useState(localStorage.getItem('ghostmark_ollama_url') || 'http://localhost:11434');
  const [ollamaModel, setOllamaModel] = useState(localStorage.getItem('ghostmark_ollama_model') || 'llama3');

  // Persist settings
  useEffect(() => {
    localStorage.setItem('ghostmark_groq_key', groqKey);
    localStorage.setItem('ghostmark_openai_key', openaiKey);
    localStorage.setItem('ghostmark_deepseek_key', deepseekKey);
    localStorage.setItem('ghostmark_gemini_key', geminiKey);
    localStorage.setItem('ghostmark_ollama_url', ollamaUrl);
    localStorage.setItem('ghostmark_ollama_model', ollamaModel);
  }, [groqKey, openaiKey, deepseekKey, geminiKey, ollamaUrl, ollamaModel]);


  const [processStatus, setProcessStatus] = useState<string>('Processing...');

  // File Upload State
  const [isHoveringFile, setIsHoveringFile] = useState(false);
  
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
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
  }, [messages, processingSessions]);

  // Processing Timer
  useEffect(() => {
    let interval: any;
    const anyProcessing = Object.values(processingSessions).some(Boolean);
    if (anyProcessing) {
      interval = setInterval(() => {
        setProcessingTime(prev => prev + 0.1);
      }, 100);
    } else {
      setProcessingTime(0);
    }
    return () => clearInterval(interval);
  }, [processingSessions]);

  // Initialize Web Worker on mount
  useEffect(() => {
    const worker = new Worker(new URL('./wasm-worker.ts', import.meta.url), { type: 'module' });
    setWasmWorker(worker);
    return () => worker.terminate();
  }, []);

  const runWasmWorker = (action: string, payload: any, fileName?: string): Promise<any> => {
    return new Promise((resolve, reject) => {
      if (!wasmWorker) return reject("Worker not initialized");
      const id = Date.now().toString() + Math.random();
      
      const listener = (e: MessageEvent) => {
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
        // Zero-copy transfer
        const buffer = payload instanceof Uint8Array ? payload.buffer : payload;
        wasmWorker.postMessage({ id, action, payload: buffer, fileName }, [buffer]);
      } else {
        wasmWorker.postMessage({ id, action, payload, fileName });
      }
    });
  };



  const checkSynthId = async (text: string): Promise<string> => {
    if (!geminiKey) return "";
    try {
      const res = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=${geminiKey}`, {
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
  };

  const checkImageSynthId = async (file: File): Promise<string> => {
    if (!geminiKey) return "";
    try {
      const arrayBuffer = await file.arrayBuffer();
      const base64 = btoa(new Uint8Array(arrayBuffer).reduce((data, byte) => data + String.fromCharCode(byte), ''));
      
      const res = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=${geminiKey}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          contents: [{ 
            role: 'user', 
            parts: [
              { inlineData: { mimeType: file.type, data: base64 } },
              { text: "Analyze this image for any AI-generated watermarks, artifacts, or cryptographic signatures like Google SynthID. Provide a short 2-sentence technical assessment of whether it appears AI-generated." }
            ] 
          }]
        })
      });
      if (!res.ok) return "API Error";
      const data = await res.json();
      return data.candidates?.[0]?.content?.parts?.[0]?.text || "Unknown";
    } catch (err) {
      return "Network Error";
    }
  };

  const handleProcessText = async (overrideText?: string) => {
    let currentSessionId = activeSessionIdRef.current;
    const textToProcess = overrideText || inputText;
    
    if (!textToProcess.trim()) return;
    
    if (!currentSessionId) {
       currentSessionId = Date.now().toString() + Math.random().toString().slice(2, 6);
       setActiveSessionId(currentSessionId);
       activeSessionIdRef.current = currentSessionId;
    }

    if (processingSessions[currentSessionId]) return;
    
    if (!overrideText) {
      setInputText('');
      if (textAreaRef.current) textAreaRef.current.style.height = 'auto';
      updateMessages((prev: Message[]) => [...prev, { role: 'user', content: textToProcess }]);
    }
    
    const controller = new AbortController();
    setAbortControllers(prev => ({ ...prev, [currentSessionId]: controller }));
    setProcessingSessions(prev => ({ ...prev, [currentSessionId]: true }));

    try {
      let currentText = textToProcess;
      
      let scoreBefore = "";
      if (useSynthIdDetect && geminiKey) {
         updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: 'Checking for SynthID watermark...' }]);
         scoreBefore = await checkSynthId(textToProcess);
      }
      
      // 1. Pre-processing
      setProcessStatus('Applying WASM Scrubbing...');
      if (wasmWorker) {
        currentText = await runWasmWorker(useShatterSynthId ? 'shatter_synthid_text' : 'sanitize_text', textToProcess);
      }

      if (controller.signal.aborted) throw new Error("Cancelled by user");

      if (llmMode === 'cloud') {
         const providerNames = { groq: 'Groq', openai: 'OpenAI', deepseek: 'DeepSeek', gemini: 'Gemini' };
         setProcessStatus(`Scrubbing via BYOK (${providerNames[cloudProvider]})...`);
         
         const systemPrompt = 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. Preserve the exact same meaning, facts, and names. Output ONLY the rewritten text, nothing else.';
         
         if (cloudProvider === 'groq' && groqKey) {
           const res = await fetch('https://api.groq.com/openai/v1/chat/completions', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${groqKey}` },
              body: JSON.stringify({
                model: 'llama3-70b-8192',
                messages: [{ role: 'system', content: systemPrompt }, { role: 'user', content: currentText }],
                temperature: 0.6,
              }),
              signal: controller.signal
           });
           if (!res.ok) throw new Error("Groq API Error.");
           const data = await res.json();
           currentText = data.choices[0].message.content;
         } else if (cloudProvider === 'openai' && openaiKey) {
           const res = await fetch('https://api.openai.com/v1/chat/completions', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${openaiKey}` },
              body: JSON.stringify({
                model: 'gpt-4o-mini',
                messages: [{ role: 'system', content: systemPrompt }, { role: 'user', content: currentText }],
                temperature: 0.6,
              }),
              signal: controller.signal
           });
           if (!res.ok) throw new Error("OpenAI API Error.");
           const data = await res.json();
           currentText = data.choices[0].message.content;
         } else if (cloudProvider === 'deepseek' && deepseekKey) {
           const res = await fetch('https://api.deepseek.com/chat/completions', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${deepseekKey}` },
              body: JSON.stringify({
                model: 'deepseek-chat',
                messages: [{ role: 'system', content: systemPrompt }, { role: 'user', content: currentText }],
                temperature: 0.6,
              }),
              signal: controller.signal
           });
           if (!res.ok) throw new Error("DeepSeek API Error.");
           const data = await res.json();
           currentText = data.choices[0].message.content;
         } else if (cloudProvider === 'gemini' && geminiKey) {
           const res = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key=${geminiKey}`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                system_instruction: { parts: [{ text: systemPrompt }] },
                contents: [{ role: 'user', parts: [{ text: currentText }] }],
                generationConfig: { temperature: 0.6 }
              }),
              signal: controller.signal
           });
           if (!res.ok) throw new Error("Gemini API Error.");
           const data = await res.json();
           currentText = data.candidates?.[0]?.content?.parts?.[0]?.text || currentText;
         } else {
           throw new Error(`Missing API Key for ${providerNames[cloudProvider]}.`);
         }
      } else if (llmMode === 'ollama') {
         setProcessStatus('Scrubbing via Local Ollama...');
         const res = await fetch(`${ollamaUrl}/api/generate`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              model: ollamaModel,
              system: 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. Preserve the exact same meaning. Output ONLY the rewritten text.',
              prompt: currentText,
              stream: false
            }),
            signal: controller.signal
         });
         if (!res.ok) throw new Error("Ollama server not responding.");
         const data = await res.json();
         currentText = data.response;
      } else if (llmMode === 'nano') {
         const ai = (window as any).ai;
         if (!ai || !ai.languageModel) {
             throw new Error("Chrome Nano is not enabled in your browser. Please enable the flag in chrome://flags/#prompt-api-for-gemini-nano");
         }
         setProcessStatus('Waking up Chrome Nano Engine...');
         const session = await ai.languageModel.create({
             systemPrompt: 'You are an expert editor. Rewrite the user\'s text to sound conversational and human. You MUST preserve the exact same meaning, names, genders, and pronouns as the original. Output only the rewritten text.'
         });
         
         if (controller.signal.aborted) throw new Error("Cancelled by user");
         
         setProcessStatus('Scrubbing via Chrome Built-in AI...');
         const rawParagraphs = currentText.split(/\n+/);
         const rewrittenParts: string[] = [];
         for (const p of rawParagraphs) {
            if (!p.trim()) continue;
            if (controller.signal.aborted) throw new Error("Cancelled by user");
            const result = await session.prompt(p);
            rewrittenParts.push(result.trim());
         }
         currentText = rewrittenParts.join('\n\n');
      }

      // 3. Post-processing
      if (useHomoglyphs && wasmWorker) {
         currentText = await runWasmWorker('sanitize_text_homoglyph', currentText);
      }

      let scoreAfter = "";
      if (useSynthIdDetect && geminiKey) {
         scoreAfter = await checkSynthId(currentText);
      }

      let finalMsg = currentText;
      if (useSynthIdDetect && geminiKey) {
         finalMsg = `[SynthID Analysis]\nBefore Scrubbing: ${scoreBefore}\nAfter Scrubbing: ${scoreAfter}\n\n[Cleaned Text]\n${currentText}`;
      }

      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: finalMsg }]);
    } catch (err: any) {
      if (err.name === 'AbortError' || err.message === 'Cancelled by user') {
         updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Cancelled.` }]);
      } else {
         updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Error: ${err.message}` }]);
      }
    } finally {
      if (currentSessionId) {
        setProcessingSessions(prev => ({ ...prev, [currentSessionId]: false }));
        setAbortControllers(prev => {
          const next = { ...prev };
          delete next[currentSessionId];
          return next;
        });
      }
    }
  };

  const handleStop = () => {
    const currentSessionId = activeSessionIdRef.current;
    if (currentSessionId && abortControllers[currentSessionId]) {
      abortControllers[currentSessionId].abort();
      setProcessingSessions(prev => ({ ...prev, [currentSessionId]: false }));
    }
  };

  const handleRetry = (idx: number) => {
    // find the previous user message
    const userMsg = [...messages].slice(0, idx).reverse().find(m => m.role === 'user');
    if (userMsg && userMsg.content) {
       // remove the current assistant message from the chat
       updateMessages((prev: Message[]) => prev.filter((_, i) => i !== idx));
       handleProcessText(userMsg.content);
    }
  };

  const processFile = async (file: File) => {
    if (!wasmWorker) return;
    const currentSessionId = activeSessionIdRef.current;
    if (!currentSessionId) return;
    
    updateMessages((prev: Message[]) => [...prev, { role: 'user', content: `Attached File: ${file.name}` }]);
    setProcessingSessions(prev => ({ ...prev, [currentSessionId]: true }));
    
    try {
      const arrayBuffer = await file.arrayBuffer();
      const originalLength = arrayBuffer.byteLength;
      
      const cleanedBuffer = await runWasmWorker('strip_file', arrayBuffer, file.name);
      const cleanedBytes = new Uint8Array(cleanedBuffer);
      const removedBytes = originalLength - cleanedBytes.length;
      
      let finalContent = `Scrubbed successfully! Removed ${removedBytes} bytes of hidden metadata/tracking data.`;
      
      // Perform SynthID image detection if requested and applicable
      if (useSynthIdDetect && geminiKey && file.type.startsWith('image/')) {
         finalContent += "\n\n*Analyzing image for AI watermarks...*";
         updateMessages((prev: Message[]) => [...prev, { 
           role: 'assistant', 
           content: finalContent,
         }]);
         
         const synthIdResult = await checkImageSynthId(file);
         finalContent = `Scrubbed successfully! Removed ${removedBytes} bytes of hidden metadata/tracking data.\n\n**SynthID / AI Vision Analysis:**\n${synthIdResult}`;
      }
      
      // Replace the loading message or add the final download message
      updateMessages((prev: Message[]) => {
         const newPrev = [...prev];
         if (useSynthIdDetect && geminiKey && file.type.startsWith('image/')) {
             newPrev.pop(); // Remove the temporary loading message
         }
         return [...newPrev, { 
           role: 'assistant', 
           content: finalContent,
           isDownloadable: true,
           fileName: file.name,
           fileBytes: cleanedBytes,
           fileType: file.type
         }];
      });
    } catch (err: any) {
      updateMessages((prev: Message[]) => [...prev, { role: 'assistant', content: `Error processing file: ${err.message}` }]);
    } finally {
      setProcessingSessions(prev => ({ ...prev, [currentSessionId]: false }));
    }
  };

  const handleFileUpload = async (file: File) => {
    if (!file) return;
    const ext = file.name.split('.').pop()?.toLowerCase();
    
    if (['png', 'jpeg', 'jpg', 'webp', 'bmp', 'gif', 'pdf', 'docx', 'epub', 'odt', 'svg'].includes(ext || '')) {
      await processFile(file);
    } else if (['txt', 'md', 'json'].includes(ext || '')) {
      const currentSessionId = activeSessionIdRef.current;
      if (!currentSessionId) return;
      
      updateMessages((prev: Message[]) => [...prev, { role: 'user', content: `Attached File: ${file.name}` }]);
      try {
         setProcessingSessions(prev => ({ ...prev, [currentSessionId]: true }));
         const text = await file.text();
         const cleaned = await runWasmWorker('sanitize_text', text);
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
         setProcessingSessions(prev => ({ ...prev, [currentSessionId]: false }));
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

  const copyToClipboard = (text: string, e: React.MouseEvent<HTMLElement>) => {
    navigator.clipboard.writeText(text);
    const btn = e.currentTarget;
    const originalHTML = btn.innerHTML;
    if (btn.tagName === 'BUTTON') {
      btn.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';
    } else {
      btn.innerHTML = 'Copied!';
    }
    setTimeout(() => { btn.innerHTML = originalHTML; }, 2000);
  };

  const renderMessageContent = (content: string) => {
    return (
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          p: ({ node, ...props }) => <p style={{ margin: '0 0 1rem 0' }} {...props} />,
          code({ node, inline, className, children, ...props }: any) {
            const str = String(children);
            if (str.startsWith('chrome://')) {
              return (
                <code
                  style={{
                    cursor: 'pointer', textDecoration: 'underline', color: 'var(--text-primary)',
                    background: 'rgba(255,255,255,0.1)', padding: '2px 4px', borderRadius: '4px'
                  }}
                  onClick={(e) => copyToClipboard(str, e as any)}
                  title="Click to copy URL"
                  {...props}
                >
                  {children}
                </code>
              );
            }
            const match = /language-(\w+)/.exec(className || '');
            return !inline && match ? (
              <div className="code-block" style={{ margin: '1rem 0', borderRadius: '8px', overflow: 'hidden', border: '1px solid var(--border)' }}>
                 <div style={{ background: 'var(--bg-surface-hover)', padding: '0.5rem 1rem', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>{match[1]}</div>
                 <pre style={{ margin: 0, padding: '1rem', overflowX: 'auto', background: 'rgba(0,0,0,0.3)' }}>
                    <code className={className} {...props}>{children}</code>
                 </pre>
              </div>
            ) : (
              <code style={{ background: 'var(--bg-surface-hover)', padding: '0.2rem 0.4rem', borderRadius: '4px' }} {...props}>{children}</code>
            );
          }
        }}
      >
        {content}
      </ReactMarkdown>
    );
  };

  // Helper for rendering the engine dropdown name
  const getEngineName = () => {
    switch (llmMode) {
      case 'none': return 'WASM Only';
      case 'cloud': 
        const names = { groq: 'Groq', openai: 'OpenAI', deepseek: 'DeepSeek', gemini: 'Gemini' };
        return `BYOK (${names[cloudProvider]})`;
      case 'ollama': return 'Local Ollama';
      case 'nano': return 'Chrome Nano Local';

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
           <button className="sidebar-btn" onClick={() => setTheme(theme === 'light' ? 'dark' : 'light')} title="Toggle Theme">
              {theme === 'light' ? <Moon size={18} /> : <Sun size={18} />}
              <span>{theme === 'light' ? 'Dark Mode' : 'Light Mode'}</span>
           </button>
           <button className="sidebar-btn" onClick={() => setShowSettings(true)}>
              <div className="avatar user" style={{ width: 24, height: 24, fontSize: '0.75rem', borderRadius: '50%', background: 'var(--bg-surface-hover)', color: 'var(--text-primary)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>G</div>
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
                    <label className="checkbox-label">
                      <input type="checkbox" checked={useShatterSynthId} onChange={(e) => setUseShatterSynthId(e.target.checked)} />
                      <span>Shatter SynthID Watermark</span>
                    </label>
                    <p className="setting-desc">Heavily perturbs token sequences by replacing synonyms and altering phrasing to destroy text watermarks.</p>
                  </div>

                  <div className="setting-group">
                    <label className="select-label">Deep Scrub Engine</label>
                    <div className="engine-grid">
                      <button className={`engine-btn ${llmMode === 'none' ? 'active' : ''}`} onClick={() => setLlmMode('none')}>WASM Only</button>
                      <button className={`engine-btn ${llmMode === 'cloud' ? 'active' : ''}`} onClick={() => setLlmMode('cloud')}>BYOK (Cloud)</button>
                      <button className={`engine-btn ${llmMode === 'ollama' ? 'active' : ''}`} onClick={() => setLlmMode('ollama')}>Ollama (10B)</button>
                      <button className={`engine-btn ${llmMode === 'nano' ? 'active' : ''}`} onClick={() => setLlmMode('nano')}>Chrome Nano</button>
                    </div>
                  </div>

                  <div className="setting-group animate-fade-in" style={{ borderTop: '1px solid var(--border-light)', paddingTop: '15px' }}>
                    <label className="select-label">SynthID Watermark Detection (Gemini API)</label>
                    <input type="password" value={geminiKey} onChange={(e) => setGeminiKey(e.target.value)} placeholder="AIza..." className="modern-input" />
                    <p className="setting-desc" style={{marginTop: '4px'}}>If set, GhostMark will query Google to verify SynthID removal.</p>
                  </div>

                  {llmMode === 'cloud' && (
                    <div className="setting-group animate-fade-in">
                      <label className="select-label">Cloud Provider</label>
                      <select 
                        className="modern-input mb-2" 
                        value={cloudProvider} 
                        onChange={(e) => setCloudProvider(e.target.value as any)}
                        style={{ appearance: 'auto' }}
                      >
                        <option value="groq">Groq (Fastest)</option>
                        <option value="openai">OpenAI (ChatGPT)</option>
                        <option value="gemini">Google Gemini</option>
                        <option value="deepseek">DeepSeek</option>
                      </select>
                      
                      <label className="select-label">{cloudProvider === 'groq' ? 'Groq' : cloudProvider === 'openai' ? 'OpenAI' : cloudProvider === 'deepseek' ? 'DeepSeek' : 'Gemini'} API Key</label>
                      {cloudProvider === 'groq' && <input type="password" value={groqKey} onChange={(e) => setGroqKey(e.target.value)} placeholder="gsk_..." className="modern-input" />}
                      {cloudProvider === 'openai' && <input type="password" value={openaiKey} onChange={(e) => setOpenaiKey(e.target.value)} placeholder="sk-..." className="modern-input" />}
                      {cloudProvider === 'deepseek' && <input type="password" value={deepseekKey} onChange={(e) => setDeepseekKey(e.target.value)} placeholder="sk-..." className="modern-input" />}
                      {cloudProvider === 'gemini' && <input type="password" value={geminiKey} onChange={(e) => setGeminiKey(e.target.value)} placeholder="AIza..." className="modern-input" />}
                    </div>
                  )}

                  {llmMode === 'ollama' && (
                    <div className="setting-group animate-fade-in">
                      <label className="select-label">Ollama Configuration</label>
                      <input type="text" value={ollamaUrl} onChange={(e) => setOllamaUrl(e.target.value)} placeholder="http://localhost:11434" className="modern-input mb-2" />
                      <input type="text" value={ollamaModel} onChange={(e) => setOllamaModel(e.target.value)} placeholder="llama3" className="modern-input" />
                    </div>
                  )}

                  {llmMode === 'nano' && (
                    <div className="warning-box animate-fade-in" style={{ fontSize: '0.85rem', flexDirection: 'column' }}>
                      <p style={{ marginBottom: '6px', color: 'var(--text-primary)' }}><strong>Chrome Nano Setup (Experimental)</strong></p>
                      <ol style={{ paddingLeft: '1.2rem', margin: 0, color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        <li>
                          Paste <code style={{ cursor: 'pointer', textDecoration: 'underline' }} onClick={(e) => copyToClipboard('chrome://flags/#prompt-api-for-gemini-nano', e as any)} title="Click to copy">chrome://flags/#prompt-api-for-gemini-nano</code> into URL bar &rarr; <strong>Enabled</strong>.
                        </li>
                        <li>
                          Paste <code style={{ cursor: 'pointer', textDecoration: 'underline' }} onClick={(e) => copyToClipboard('chrome://flags/#optimization-guide-on-device-model', e as any)} title="Click to copy">chrome://flags/#optimization-guide-on-device-model</code> into URL bar &rarr; <strong>Enabled BypassPerfRequirement</strong>.
                        </li>
                        <li>Click Relaunch Chrome.</li>
                        <li>Go to <code style={{ cursor: 'pointer', textDecoration: 'underline' }} onClick={(e) => copyToClipboard('chrome://components', e as any)} title="Click to copy">chrome://components</code>, find <strong>Optimization Guide On Device Model</strong>, and click <strong>Check for update</strong>.</li>
                      </ol>
                    </div>
                  )}

                  <div className="setting-group animate-fade-in" style={{ borderTop: '1px solid var(--border-light)', paddingTop: '15px' }}>
                    <label className="select-label" style={{ marginBottom: '10px' }}>Resources & Links</label>
                    <div style={{ display: 'grid', gap: '8px' }}>
                      <button onClick={() => { setShowSettings(false); fileInputRef.current?.click(); }} className="suggest-btn" style={{ padding: '0.6rem 1rem', background: 'var(--bg-surface)', border: '1px solid var(--border)', borderRadius: '8px', display: 'flex', alignItems: 'center', gap: '0.75rem', justifyContent: 'flex-start', color: 'var(--text-primary)', transition: 'all 0.2s', width: '100%' }}>
                        <FileCode size={16} color="var(--text-secondary)" />
                        <span style={{ fontSize: '0.85rem', fontWeight: 500 }}>Scrub a Document</span>
                      </button>
                      <a href="https://github.com/kilopal/GhostMark/tree/main/extension" target="_blank" rel="noreferrer" className="suggest-btn" style={{ padding: '0.6rem 1rem', background: 'var(--bg-surface)', border: '1px solid var(--border)', borderRadius: '8px', display: 'flex', alignItems: 'center', gap: '0.75rem', justifyContent: 'flex-start', color: 'var(--text-primary)', transition: 'all 0.2s', textDecoration: 'none', width: '100%' }}>
                        <Globe size={16} color="var(--text-secondary)" />
                        <span style={{ fontSize: '0.85rem', fontWeight: 500 }}>Browser Extension</span>
                      </a>
                      <a href="https://github.com/kilopal/GhostMark/tree/main/cli" target="_blank" rel="noreferrer" className="suggest-btn" style={{ padding: '0.6rem 1rem', background: 'var(--bg-surface)', border: '1px solid var(--border)', borderRadius: '8px', display: 'flex', alignItems: 'center', gap: '0.75rem', justifyContent: 'flex-start', color: 'var(--text-primary)', transition: 'all 0.2s', textDecoration: 'none', width: '100%' }}>
                        <Terminal size={16} color="var(--text-secondary)" />
                        <span style={{ fontSize: '0.85rem', fontWeight: 500 }}>Use the Rust CLI</span>
                      </a>
                    </div>
                  </div>

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
            <div className="empty-state animate-fade-in" style={{ display: 'flex', flexDirection: 'column', minHeight: '100%', flex: 1, padding: '2rem 1rem' }}>
              <div style={{ margin: 'auto 0', width: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                <div className="empty-logo" style={{ marginBottom: '1rem', fontSize: '2rem' }}>👻</div>
              <h2 style={{ fontSize: '1.75rem', fontWeight: 600, marginBottom: '0.5rem', textAlign: 'center', color: 'var(--text-primary)' }}>How can I scrub your data today?</h2>
              <p className="app-description" style={{ textAlign: 'center', color: 'var(--text-secondary)', maxWidth: '540px', marginBottom: '2rem', lineHeight: '1.6', fontSize: '0.95rem' }}>
                GhostMark 👻 A blazing-fast, memory-safe tool written in Rust to strip Anthropic, OpenAI, and EU-mandated AI watermarks (C2PA &amp; Unicode) from text and images.
              </p>
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
                    <div className="prose">{renderMessageContent(msg.content)}</div>
                    
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
                        {!msg.isDownloadable && (
                          <button onClick={() => handleRetry(idx)} className="action-btn icon-only" title="Retry">
                            <RotateCcw size={16} />
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
          {processingSessions[activeSessionId || ''] && (
             <div className="message-row assistant">
               <div className="message-content animate-fade-in">
                 <div className="avatar assistant pulse-bg">👻</div>
                 <div style={{ flex: 1, minWidth: 0 }}>
                   <div style={{ paddingTop: '6px', color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
                     <span className="animate-pulse">{processStatus} ({processingTime.toFixed(1)}s)</span>
                   </div>
                   <div className="msg-actions" style={{ marginTop: '12px' }}>
                     <button onClick={handleStop} className="action-btn" style={{ color: 'var(--danger)', borderColor: 'rgba(239, 68, 68, 0.2)' }}>
                       <Square size={14} fill="currentColor" /> Stop Generation
                     </button>
                   </div>
                 </div>
               </div>
             </div>
          )}
          <div ref={feedEndRef} style={{ height: '180px' }} />
        </div>

        {/* Input Box Area */}
        <div className="input-container">
          <div className="toggles-container">
            <label className="toggle-row" title="Instantly scrub text using WASM without AI paraphrasing">
              <input type="checkbox" className="toggle-checkbox" checked={llmMode === 'none'} onChange={() => setLlmMode('none')} />
              <div className="toggle-track"></div>
              <span className="toggle-label">WASM Fast</span>
            </label>
            <label className="toggle-row" title="Rewrite text using Cloud AI APIs, local Ollama, or Chrome Nano">
              <input type="checkbox" className="toggle-checkbox" checked={llmMode === 'cloud' || llmMode === 'ollama' || llmMode === 'nano'} onChange={(e) => {
                if (e.target.checked) {
                  setLlmMode('cloud');
                  setShowSettings(true);
                } else {
                  setLlmMode('none');
                }
              }} />
              <div className="toggle-track"></div>
              <span className="toggle-label">Deep Scrub</span>
            </label>
            <label className="toggle-row" title="Check SynthID watermarks with Gemini API">
              <input type="checkbox" className="toggle-checkbox" checked={useSynthIdDetect} onChange={(e) => {
                if (e.target.checked && !geminiKey) {
                  setShowSettings(true);
                } else {
                  setUseSynthIdDetect(e.target.checked);
                }
              }} />
              <div className="toggle-track"></div>
              <span className="toggle-label">SynthID Detect</span>
            </label>
          </div>
          <div className="input-wrapper">
            <input type="file" ref={imageInputRef} style={{ display: 'none' }} accept="image/*" onChange={(e) => { if (e.target.files && e.target.files[0]) handleFileUpload(e.target.files[0]); }} />
            <input type="file" ref={fileInputRef} style={{ display: 'none' }} accept=".pdf,.docx,.epub,.odt,.svg,.txt,.md,.json,application/pdf,text/plain" onChange={(e) => { if (e.target.files && e.target.files[0]) handleFileUpload(e.target.files[0]); }} />
            
            <button className="attach-btn" onClick={() => imageInputRef.current?.click()} title="Attach Image">
              <Image size={20} />
            </button>
            <button className="attach-btn" onClick={() => fileInputRef.current?.click()} title="Attach Document">
              <FileText size={20} />
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
              disabled={processingSessions[activeSessionId || ''] || !wasmWorker}
            />

            {processingSessions[activeSessionId || ''] ? (
              <button className="submit-btn" onClick={handleStop} style={{ backgroundColor: 'var(--text-primary)', color: 'var(--bg-surface)' }}>
                <Square size={16} strokeWidth={2.5} fill="currentColor" />
              </button>
            ) : (
              <button className="submit-btn" onClick={() => handleProcessText()} disabled={!inputText.trim() || !wasmWorker}>
                <ArrowUp size={20} strokeWidth={2.5} />
              </button>
            )}
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
