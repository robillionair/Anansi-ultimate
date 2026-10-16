import { createServer } from 'node:net';
export async function assertPortsFree(ports) {
  for(const port of ports)await new Promise((resolve,reject)=>{
    const server=createServer();
    server.once('error',()=>reject(Error(`Verification port ${port} is already in use. Stop the other test or service and retry.`)));
    server.listen({host:'127.0.0.1',port,exclusive:true},()=>server.close(error=>error?reject(error):resolve()));
  });
}
