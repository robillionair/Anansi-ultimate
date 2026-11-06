import { useEffect, useState } from 'react';
import type { ArtifactRecord, Client } from '@anansi/contracts';
import { SwarmtankPanel } from '../Swarmtank.tsx';

export function LarpSwarmtank({ client, workspaceId }: { client?: Client; workspaceId?: string }) {
  const [projects, setProjects] = useState<{ id: string; title: string }[] | null>(null);
  const [creating, setCreating] = useState(false);
  const [title, setTitle] = useState('');
  const [project, setProject] = useState('');
  const [artifacts, setArtifacts] = useState<ArtifactRecord[] | null>(null);
  const [arena, setArena] = useState<HTMLDivElement | null>(null);
  const [notice, setNotice] = useState<{ message: string; error: boolean } | null>(null);

  useEffect(() => {
    if (!client || !workspaceId) return;
    let active = true;
    void client.listMemories(workspaceId, { kind: 'project' }).then(result => {
      if (active) setProjects(result.memories);
    }).catch(() => {
      if (active) setNotice({ message: 'Could not load projects. Reopen SwarmTank to retry.', error: true });
    });
    return () => { active = false; };
  }, [client, workspaceId]);

  useEffect(() => {
    if (!client || !workspaceId || !project) return;
    let active = true;
    void client.listProjectArtifacts(workspaceId, project).then(result => {
      if (active) setArtifacts(result.artifacts);
    }).catch(() => {
      if (active) setNotice({ message: 'Could not load this project. Choose another project or reopen SwarmTank to retry.', error: true });
    });
    return () => { active = false; };
  }, [client, workspaceId, project]);

  return <div className="larp-swarmtank">
    <div className="larp-swarm-stage" ref={setArena} />
    <div className="larp-swarm-idle" aria-hidden="true">
      <div className="larp-swarm-orbit" />
      <span>SYNTHETIC BUYER ARENA</span>
      <strong>Awaiting simulation</strong>
      <span>RUN OR OPEN A SAVED SIMULATION</span>
    </div>
    <div className="larp-swarm-console" aria-label="SwarmTank controls">
      <header><span>08 / SCENARIO LAB</span><h2>SwarmTank controls</h2><p>Simulated personas for scenario planning.</p></header>
      {client && workspaceId ? <>
        <label htmlFor="larp-swarm-project">Project</label>
        <select id="larp-swarm-project" disabled={creating} value={project} onChange={event => {
          setArtifacts(null);
          setNotice(null);
          setProject(event.target.value);
        }}>
          <option value="">Choose a project</option>
          {projects?.map(item => <option key={item.id} value={item.id}>{item.title}</option>)}
        </select>
        {projects?.length === 0 && <form onSubmit={async event => {
          event.preventDefault();
          if (creating || !title.trim()) return;
          setCreating(true);
          setNotice(null);
          try {
            const { memory } = await client.createMemory(workspaceId, { title: title.trim(), body: 'Created for SwarmTank.' }, 'project');
            setProjects([memory]);
            setProject(memory.id);
          } catch {
            setNotice({ message: 'Could not create the project. Try again.', error: true });
          } finally {
            setCreating(false);
          }
        }}>
          <label htmlFor="larp-swarm-project-title">New project title</label>
          <input id="larp-swarm-project-title" value={title} onChange={event => setTitle(event.target.value)} maxLength={120} disabled={creating} required />
          <button type="submit" disabled={creating || !title.trim()}>Create project</button>
        </form>}
        {projects === null && !notice && <p role="status">Loading projects…</p>}
        {project && artifacts === null && !notice && <p role="status">Loading saved runs…</p>}
        {notice && <p className="larp-swarm-notice" role={notice.error ? 'alert' : 'status'}>{notice.message}</p>}
        {project && artifacts && arena && <ProjectSimulation key={project} client={client} workspaceId={workspaceId} project={project} initialArtifacts={artifacts} arena={arena} />}
      </> : <p>Open a workspace to run a simulation.</p>}
    </div>
  </div>;
}

function ProjectSimulation({ client, workspaceId, project, initialArtifacts, arena }: {
  client: Client;
  workspaceId: string;
  project: string;
  initialArtifacts: ArtifactRecord[];
  arena: HTMLDivElement;
}) {
  const [artifacts, setArtifacts] = useState(initialArtifacts);
  const [notice, setNotice] = useState<{ message: string; error: boolean } | null>(null);
  return <>
    {notice && <p className="larp-swarm-notice" role={notice.error ? "alert" : "status"}>{notice.message}</p>}
    <SwarmtankPanel client={client} workspaceId={workspaceId} projectId={project} artifacts={artifacts} arenaHost={arena} notify={(message, error) => setNotice({ message, error })} busy={false} onSaved={async () => {
    const result = await client.listProjectArtifacts(workspaceId, project);
    setArtifacts(result.artifacts);
    }} />
  </>;
}
