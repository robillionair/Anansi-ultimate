import { useState } from 'react';
import type { Client, MemoryDetails, MemoryMetadata } from '@anansi/contracts';

export function Search({client,workspaceId}:{client:Client;workspaceId:string}) {
  const [query,setQuery]=useState(''),[kind,setKind]=useState('');
  const [archived,setArchived]=useState(false),[expired,setExpired]=useState(false);
  const [results,setResults]=useState<MemoryMetadata[]>([]),[current,setCurrent]=useState<MemoryDetails|null>(null);
  const [busy,setBusy]=useState(false),[error,setError]=useState(''),[searched,setSearched]=useState(false);
  const [offset,setOffset]=useState(0),[next,setNext]=useState<number|null>(null);
  function clear(){setResults([]);setCurrent(null);setError('');setSearched(false);setOffset(0);setNext(null);}
  async function search(at=0){
    setBusy(true);setError('');setCurrent(null);setResults([]);setSearched(false);setNext(null);
    try{const result=await client.listMemories(workspaceId,{query,kind:kind||undefined,includeArchived:archived,includeExpired:expired,offset:at});setResults(result.memories);setOffset(at);setNext(result.nextOffset??null);setSearched(true);}
    catch(e){setError(e instanceof Error?e.message:'Search unavailable. Retry.');}
    finally{setBusy(false);}
  }
  async function open(record:MemoryMetadata){setBusy(true);setError('');setCurrent(null);try{setCurrent(await client.readMemory(workspaceId,record.id,record.revisionId));}catch(e){setError(e instanceof Error?e.message:'Record unavailable. Search again.');}finally{setBusy(false);}}
  return <section className="card memory" aria-labelledby="search-title" aria-busy={busy}>
    <h2 id="search-title">Search saved titles</h2>
    <p>Search record titles in this workspace. All words must match. Bodies and semantic search are not included. Nothing is sent to an AI provider.</p>
    <form onSubmit={e=>{e.preventDefault();void search();}}><fieldset disabled={busy}>
      <label htmlFor="search-query">Title keywords</label><input id="search-query" value={query} onChange={e=>{clear();setQuery(e.target.value);}} maxLength={256} required autoComplete="off"/>
      <label htmlFor="search-kind">Record type</label><select id="search-kind" value={kind} onChange={e=>{clear();setKind(e.target.value);}}><option value="">All record types</option>{['memory','note','project','task','person','organization','decision','event','document'].map(k=><option key={k} value={k}>{k}</option>)}</select>
      <label className="check-label"><input type="checkbox" checked={archived} onChange={e=>{clear();setArchived(e.target.checked);}}/>Include archived search results</label>
      <label className="check-label"><input type="checkbox" checked={expired} onChange={e=>{clear();setExpired(e.target.checked);}}/>Include expired search results</label>
      <button type="submit">Search titles</button>
    </fieldset></form>
    <p className="help">Up to eight words, 256 UTF-8 bytes. Searches are limited to 10,000 eligible records and 32 MiB of metadata; narrow the record type if needed. Sealed records remain unavailable.</p>
    {busy&&<p role="status">Searching or opening a saved revision…</p>}{error&&<p role="alert" className="error">{error}</p>}
    {searched&&<><p role="status">{results.length?`${results.length} title matches on this page.`:'No matching titles. Try different words or filters.'}</p>
      <ul aria-label="Title search results">{results.map(record=><li key={record.id}><button className="secondary" disabled={busy} onClick={()=>void open(record)}>{record.title}</button><p>{record.kind} · {record.status} · {record.privacy} · Revision {record.version}</p></li>)}</ul>
      <div className="actions"><button disabled={busy||offset===0} onClick={()=>void search(Math.max(0,offset-50))}>Previous search results</button><button disabled={busy||next===null} onClick={()=>void search(next!)}>Next search results</button></div></>}
    {current&&<section aria-label="Search result revision"><h3>{current.memory.title}</h3><p>Revision {current.memory.version} · {current.memory.provenance} · {current.memory.validationStatus}</p><p>Sources: {current.memory.sourceReferences.join(', ')||'none recorded'}</p><p>Record: <code>{current.memory.id}</code> · Revision: <code>{current.memory.revisionId}</code></p><pre>{current.body}</pre></section>}
  </section>;
}
