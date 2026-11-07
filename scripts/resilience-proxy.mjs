#!/usr/bin/env node
/**
 * LAB-RESILIENCE Fault Injection TCP Proxy for Zonendpoint
 * Tier 2 Native Verification Harness
 *
 * Zero-dependency: uses only built-in node:net
 * DEFENDER-SAFE: script file, loopback-only (127.0.0.1), no dynamic code evaluation
 *
 * Usage:
 *   node scripts/resilience-proxy.mjs [--listen=13399] [--target=13310] [--control=13398] [--fault=passthrough]
 *
 * Control Commands (send to control port 13398):
 *   SET FAULT=<passthrough|reset-instream|stall-instream|corrupt-response|bump-identity|drop-connect>
 *   STATUS
 *   QUIT
 */

import net from 'node:net';

// Parse command line arguments
const args = Object.fromEntries(
  process.argv.slice(2).map(arg => {
    const [k, v] = arg.replace(/^--/, '').split('=');
    return [k, v ?? 'true'];
  })
);

const LISTEN_HOST = '127.0.0.1';
const LISTEN_PORT = parseInt(args.listen || '13399', 10);
const TARGET_HOST = '127.0.0.1';
const TARGET_PORT = parseInt(args.target || '13310', 10);
const CONTROL_PORT = parseInt(args.control || '13398', 10);

let currentFault = args.fault || 'passthrough';
let connectionCount = 0;

function log(msg) {
  const ts = new Date().toISOString().slice(11, 23);
  console.log(`[${ts}] [ResilienceProxy] ${msg}`);
}

// Control server for dynamic fault switching
const controlServer = net.createServer(socket => {
  socket.on('data', data => {
    const text = data.toString('utf8').trim();
    if (text.startsWith('SET FAULT=')) {
      const mode = text.slice(10).trim();
      currentFault = mode;
      log(`Fault mode changed dynamically to: ${currentFault}`);
      socket.write(`OK FAULT=${currentFault}\n`);
    } else if (text === 'STATUS') {
      socket.write(`STATUS FAULT=${currentFault} CONNS=${connectionCount}\n`);
    } else if (text === 'QUIT') {
      socket.write('BYE\n');
      process.exit(0);
    } else {
      socket.write(`ERR unknown command: ${text}\n`);
    }
  });
});

controlServer.listen(CONTROL_PORT, LISTEN_HOST, () => {
  log(`Control server listening on ${LISTEN_HOST}:${CONTROL_PORT}`);
});

// Proxy server
const proxyServer = net.createServer(clientSocket => {
  const connId = ++connectionCount;
  const fault = currentFault;
  log(`[Conn #${connId}] Accepted connection from ${clientSocket.remotePort} (Fault mode: ${fault})`);

  if (fault === 'drop-connect') {
    log(`[Conn #${connId}] FAULT drop-connect: immediately destroying socket`);
    clientSocket.destroy();
    return;
  }

  let upstreamSocket = null;
  let clientBuffer = Buffer.alloc(0);
  let commandDetected = null;

  function connectUpstream(onConnect) {
    upstreamSocket = net.connect({ host: TARGET_HOST, port: TARGET_PORT }, () => {
      log(`[Conn #${connId}] Connected to upstream clamd at ${TARGET_HOST}:${TARGET_PORT}`);
      if (onConnect) onConnect();
    });

    upstreamSocket.on('error', err => {
      log(`[Conn #${connId}] Upstream error: ${err.message}`);
      clientSocket.destroy();
    });

    upstreamSocket.on('close', () => {
      clientSocket.end();
    });
  }

  clientSocket.on('data', chunk => {
    if (!commandDetected) {
      clientBuffer = Buffer.concat([clientBuffer, chunk]);
      const nulIdx = clientBuffer.indexOf(0x00);
      if (nulIdx !== -1) {
        commandDetected = clientBuffer.subarray(0, nulIdx + 1).toString('latin1');
        log(`[Conn #${connId}] Detected command: ${JSON.stringify(commandDetected)}`);

        // Handle fault based on detected command
        if (commandDetected.startsWith('zVERSION')) {
          if (fault === 'bump-identity') {
            log(`[Conn #${connId}] FAULT bump-identity: intercepting zVERSION`);
            connectUpstream(() => {
              upstreamSocket.once('data', verData => {
                const verStr = verData.toString('utf8');
                log(`[Conn #${connId}] Upstream real version: ${verStr.trim()}`);
                // Replace signature number: increment or bump e.g. 28175 -> 28176
                const bumped = verStr.replace(/\/(\d+)\//, (_, num) => `/${parseInt(num, 10) + 1}/`);
                log(`[Conn #${connId}] Returning bumped version: ${bumped.trim()}`);
                clientSocket.write(Buffer.from(bumped, 'utf8'));
              });
              upstreamSocket.write(clientBuffer);
            });
            return;
          }
        }

        if (commandDetected.startsWith('zINSTREAM')) {
          if (fault === 'reset-instream') {
            log(`[Conn #${connId}] FAULT reset-instream: destroying client socket mid-stream`);
            clientSocket.destroy();
            return;
          }

          if (fault === 'stall-instream') {
            log(`[Conn #${connId}] FAULT stall-instream: holding stream open without response`);
            // Do not forward, do not reply
            return;
          }

          if (fault === 'corrupt-response') {
            log(`[Conn #${connId}] FAULT corrupt-response: forwarding to upstream, will mutate reply`);
            connectUpstream(() => {
              upstreamSocket.once('data', () => {
                log(`[Conn #${connId}] Intercepted upstream verdict, replying with corrupt frame 'garbage OK\\0'`);
                clientSocket.write(Buffer.from('garbage OK\0', 'utf8'));
                clientSocket.end();
              });
              upstreamSocket.write(clientBuffer);
            });
            return;
          }
        }

        // Passthrough or standard forward
        connectUpstream(() => {
          upstreamSocket.pipe(clientSocket);
          upstreamSocket.write(clientBuffer);
        });
        return;
      }
    } else {
      if (upstreamSocket && !upstreamSocket.destroyed) {
        upstreamSocket.write(chunk);
      }
    }
  });

  clientSocket.on('error', err => {
    log(`[Conn #${connId}] Client socket error: ${err.message}`);
    if (upstreamSocket) upstreamSocket.destroy();
  });

  clientSocket.on('close', () => {
    log(`[Conn #${connId}] Client disconnected`);
    if (upstreamSocket) upstreamSocket.end();
  });
});

proxyServer.listen(LISTEN_PORT, LISTEN_HOST, () => {
  log(`=== Zonendpoint LAB-RESILIENCE Fault Proxy Running ===`);
  log(`Listening: ${LISTEN_HOST}:${LISTEN_PORT}`);
  log(`Forwarding to: ${TARGET_HOST}:${TARGET_PORT}`);
  log(`Control port: ${LISTEN_HOST}:${CONTROL_PORT}`);
  log(`Initial fault mode: ${currentFault}`);
  log(`Ready for Qeen Tier 2 visible verification.`);
});
