export function injectPluginPolicy(html: string, capabilities: { webgl: boolean; worker: boolean }) {
  const policy = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline' data: blob:; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; media-src data: blob:; connect-src 'none'; worker-src blob:; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; navigate-to 'none'">`;
  const bootstrap = `<script>(function(){
    const listeners=[];let sessionId='';let ready=false;let startupError='';
    const sendError=message=>{if(sessionId)parent.postMessage({version:1,sessionId,type:'error',message:String(message).slice(0,500)},'*')};
    const describeError=event=>event.error&&event.error.message?event.error.message:(event.message||'插件脚本错误')+(event.filename?(' @ '+event.filename+':'+event.lineno+':'+event.colno):'');
    addEventListener('error',event=>{if(ready)return;startupError=describeError(event);sendError(startupError);},true);
    addEventListener('unhandledrejection',event=>{startupError=event.reason&&event.reason.message?event.reason.message:String(event.reason||'插件 Promise 错误');sendError(startupError);},true);
    window.open=()=>null;
    Object.defineProperty(HTMLFormElement.prototype,'submit',{value:function(){throw new Error('form submit blocked')},configurable:true});
    const originalCreateElement=document.createElement.bind(document);document.createElement=function(name,...args){const element=originalCreateElement(name,...args);if(String(name).toLowerCase()==='form')Object.defineProperty(element,'submit',{value:function(){throw new Error('form submit blocked')},configurable:true});return element};
    window.showOpenFilePicker=undefined;window.showSaveFilePicker=undefined;window.showDirectoryPicker=undefined;
    try{Object.defineProperty(window,'localStorage',{get(){throw new Error('storage blocked')},configurable:true});Object.defineProperty(window,'sessionStorage',{get(){throw new Error('storage blocked')},configurable:true});}catch{}
    try{Object.defineProperty(Storage.prototype,'setItem',{value:function(){throw new Error('storage blocked')},configurable:true});Object.defineProperty(Storage.prototype,'getItem',{value:function(){throw new Error('storage blocked')},configurable:true});}catch{}
    addEventListener('dragover',event=>{event.preventDefault();event.stopPropagation();},true);
    addEventListener('drop',event=>{event.preventDefault();event.stopPropagation();},true);
    ${capabilities.worker ? "" : "window.Worker=undefined;window.SharedWorker=undefined;"}
    ${capabilities.webgl ? "" : "const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){if(type==='webgl'||type==='webgl2')return null;return original.call(this,type,...args)};"}
    const sendReady=()=>{if(sessionId&&ready)parent.postMessage({version:1,sessionId,type:'ready'},'*')};
    window.motionCue={onPlay(fn){listeners.push(fn)},ready(){ready=true;sendReady()},complete(){parent.postMessage({version:1,sessionId,type:'complete'},'*')},error(message){sendError(message)},sound(soundId){parent.postMessage({version:1,sessionId,type:'sound',soundId},'*')}};
    addEventListener('message',event=>{const message=event.data||{};if(message.version!==1)return;if(message.type==='play'){sessionId=message.sessionId;if(startupError){sendError(startupError);return;}sendReady();listeners.forEach(fn=>fn(message.payload));}if(message.type==='stop')dispatchEvent(new Event('motioncue-stop'));});
  })();<${"/"}script>`;
  return html.includes("<head") ? html.replace(/<head[^>]*>/i, (head) => `${head}${policy}${bootstrap}`) : `${policy}${bootstrap}${html}`;
}
