import { spawn } from 'node:child_process';
const source = `
import net from 'node:net';
const server = net.createServer(socket => socket.end());
server.listen(0,'127.0.0.1',()=>console.log(JSON.stringify({pid:process.pid,port:server.address().port})));
`;
const child = spawn(process.execPath, ['--input-type=module', '-e', source], { stdio: ['ignore', 'pipe', 'inherit'] });
child.stdout.pipe(process.stdout);
setInterval(() => {}, 1000);
