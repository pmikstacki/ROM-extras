import tls from 'node:tls';
import fs from 'node:fs';
export function receiver({key,cert,events}) {
 const sockets=new Set();
 const server=tls.createServer({key,cert,minVersion:'TLSv1.2'},socket=>{
  socket.setNoDelay(true);
  sockets.add(socket);socket.on('error',()=>{});socket.on('close',()=>sockets.delete(socket));
  let pending='';let mode='commands';let authorized=false;let body=[];let silentQuit=false;let wrongData=false;let challengeReject=false;
  socket.write('220 smtp.fixture.test ready\r\n');
  const event=value=>fs.appendFileSync(events,JSON.stringify(value)+'\n');
  socket.on('data',bytes=>{
   pending+=bytes.toString('utf8');
   if(pending.length>1048576){socket.destroy();return;}
   while(pending.includes('\r\n')) {
    const end=pending.indexOf('\r\n');const line=pending.slice(0,end);pending=pending.slice(end+2);
    if(mode==='challenge'){mode='commands';silentQuit=true;socket.write('535 authentication denied\r\n');continue;}
    if(mode==='data') {
     if(line!=='.'){body.push(line.startsWith('..')?line.slice(1):line);continue;}
     mode='commands';const wire=body.join('\r\n');body=[];
     const subject=/^Subject: (.*)$/m.exec(wire)?.[1].trim();
     const id=/^Message-ID: (.*)$/m.exec(wire)?.[1].trim();
     if(!subject||!id||wire.includes('synthetic-password')){socket.destroy();return;}
     event({subject,id,wire,accepted:!['transient','permanent','rejectquit','wrongpositive'].includes(subject)});
     if(subject==='loss'){socket.destroy();return;}
     if(['slow','cancel','held'].includes(subject)){continue;}
     if(subject==='rejectquit'){silentQuit=true;socket.write('450 explicit rejection\r\n');continue;}
     if(subject==='transient'){socket.write('450 temporary fixture rejection\r\n');continue;}
     if(subject==='permanent'){socket.write('550 permanent fixture rejection\r\n');continue;}
     if(subject==='wrongpositive'){socket.write('251 not a DATA acknowledgment\r\n');continue;}
     socket.write('250 stored fixture message\r\n');continue;
    }
    if(line.startsWith('EHLO ')) {
     wrongData=line==='EHLO wrongdata.example.invalid';
     if(line==='EHLO rejectehlo.example.invalid'){silentQuit=true;socket.write('450 EHLO denied\r\n');continue;}
     challengeReject=line==='EHLO challenge.example.invalid';
     if(line==='EHLO overflow.example.invalid'){socket.write('250-'+ 'A'.repeat(100000));continue;}
     socket.write('250-smtp.fixture.test\r\n250-AUTH PLAIN\r\n250 SIZE 1048576\r\n');continue;
    }
    if(line.startsWith('AUTH PLAIN ')) {
     if(challengeReject){silentQuit=true;mode='challenge';socket.write('334 \r\n');continue;}
     const parts=Buffer.from(line.slice(11),'base64').toString().split('\0');
     authorized=parts.length===3&&parts[1]==='fixture'&&parts[2]==='synthetic-password';
     socket.write(authorized?'235 authenticated\r\n':'535 rejected fixture credentials\r\n');continue;
    }
    if(line==='QUIT'&&silentQuit){continue;}
    if(line==='QUIT'){socket.end('221 bye\r\n');continue;}
    if(!authorized){socket.write('530 authenticate first\r\n');continue;}
    if(line==='MAIL FROM:<sender@example.invalid>'){socket.write('250 sender accepted\r\n');continue;}
    if(line==='RCPT TO:<to@example.invalid>'){socket.write('250 recipient accepted\r\n');continue;}
    if(line==='DATA'){mode='data';socket.write(wrongData?'250 invalid DATA readiness\r\n':'354 send data\r\n');continue;}
    socket.write('500 unsupported fixture command\r\n');
   }
  });
 });
 server.on('tlsClientError',()=>{});
 return {server,close:async()=>{for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve));}};
}
