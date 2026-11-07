import os,socket,struct,time
p='/var/lib/clamav/zonendpoint-qa-8e8204a.hsb'
assert p=='/var/lib/clamav/zonendpoint-qa-8e8204a.hsb'
os.unlink(p)
with socket.create_connection(('127.0.0.1',3310),timeout=10) as s:s.sendall(b'zRELOAD\0');print(s.recv(1024).decode().strip('\0'))
payload=bytes(range(256))*4+b'Zonendpoint harmless QA custom signature 8e8204a'
for _ in range(40):
 with socket.create_connection(('127.0.0.1',3310),timeout=10) as s:
  s.sendall(b'zINSTREAM\0'+struct.pack('!I',len(payload))+payload+struct.pack('!I',0));r=s.recv(1024)
 if r==b'stream: OK\0':print('CLEANUP VERIFIED: '+r.decode().strip('\0'));break
 time.sleep(.5)
else:raise RuntimeError('Custom lab signature remained loaded')
