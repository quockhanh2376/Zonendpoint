import os,hashlib,socket,struct,time
payload=bytes(range(256))*4+b'Zonendpoint harmless QA custom signature 8e8204a'
p='/var/lib/clamav/zonendpoint-qa-8e8204a.hsb'
fd=os.open(p,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o644)
with os.fdopen(fd,'w') as f:f.write(hashlib.sha256(payload).hexdigest()+':'+str(len(payload))+':Zonendpoint.LabOnly.Test\n')
with socket.create_connection(('127.0.0.1',3310),timeout=10) as s:
 s.sendall(b'zRELOAD\0');print(s.recv(1024).decode().strip('\0'))
for _ in range(40):
 with socket.create_connection(('127.0.0.1',3310),timeout=10) as s:
  s.sendall(b'zINSTREAM\0'+struct.pack('!I',len(payload))+payload+struct.pack('!I',0));r=s.recv(1024)
 if b'FOUND' in r:print(r.decode().strip('\0'));break
 time.sleep(.5)
else:raise RuntimeError('Lab signature did not load')
