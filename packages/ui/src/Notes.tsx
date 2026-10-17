import type { Client } from "@anansi/contracts";
import { Memory } from "./Memory";
export function Notes(props:{client:Client;workspaceId:string}) { return <Memory {...props} kind="note"/>; }
