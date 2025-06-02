const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const fs = require('fs');
const { blake2AsHex } = require('@polkadot/util-crypto');
const { hexToU8a } = require('@polkadot/util');
const path = require('path');

// Utility: Sleep for ms milliseconds
function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

// Utility: Print extrinsic status and events
function printExtrinsicStatus(label, status, events, dispatchError, api) {
    if (status.isInBlock) {
        console.log(`[${label}] Included at blockHash`, status.asInBlock.toHex());
    } else if (status.isFinalized) {
        console.log(`[${label}] Finalized at blockHash`, status.asFinalized.toHex());
    }
    if (dispatchError) {
        if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            const { docs, name, section } = decoded;
            console.error(`[${label}] DispatchError: ${section}.${name}: ${docs.join(' ')}`);
        } else {
            console.error(`[${label}] DispatchError: ${dispatchError.toString()}`);
        }
    } else {
        events.forEach(({ event: { data, method, section } }) => {
            console.log(`[${label}] Event: ${section}.${method} ${data.toString()}`);
        });
    }
}

// Send extrinsic with retries and debug output
async function sendWithRetry(api, extrinsic, sudoAccount, label, maxRetries = 5) {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
            console.log(`[${label}] Attempt ${attempt}`);
            await new Promise((resolve, reject) => {
                extrinsic.signAndSend(sudoAccount, ({ status, events, dispatchError }) => {
                    printExtrinsicStatus(label, status, events, dispatchError, api);
                    if (status.isInBlock || status.isFinalized) {
                        if (dispatchError) {
                            reject(new Error('ExtrinsicFailed'));
                        } else {
                            resolve();
                        }
                    }
                }).catch(reject);
            });
            return true;
        } catch (err) {
            console.error(`[${label}] Error:`, err.message || err);
            if (attempt < maxRetries) {
                await sleep(2000);
            }
        }
    }
    return false;
}

// Minimal sudo test: sudo.sudo(system.remark)
async function testSudoRemark(api, sudoAccount) {
    const remarkCall = api.tx.system.remark('0x1234');
    const sudoCall = api.tx.sudo.sudo(remarkCall);
    console.log('Testing sudo.sudo(system.remark)...');
    console.log('remarkCall method hex:', remarkCall.method.toHex());
    console.log('sudoCall method hex:', sudoCall.method.toHex());
    await new Promise((resolve, reject) => {
        sudoCall.signAndSend(sudoAccount, ({ status, events, dispatchError }) => {
            printExtrinsicStatus('TestSudo', status, events, dispatchError, api);
            if (status.isInBlock || status.isFinalized) {
                resolve();
            }
        }).catch(reject);
    });
    console.log('Test sudo.sudo(system.remark) completed.');
}

// Detect sudo/proxy status for the account
async function getSudoProxyStatus(api, sudoAccount) {
    const sudoKey = (await api.query.sudo.key()).toString();
    const isSudo = sudoAccount.address === sudoKey;
    let isProxySudo = false;
    if (!isSudo) {
        const proxies = await api.query.proxy.proxies(sudoKey);
        if (proxies[0].length > 0) {
            for (const proxy of proxies[0]) {
                if (proxy.delegate.toString() === sudoAccount.address) {
                    isProxySudo = true;
                    break;
                }
            }
        }
    }
    console.log(`Sudo key: ${sudoKey}`);
    console.log(`Using account: ${sudoAccount.address}`);
    console.log(`Is sudo: ${isSudo}`);
    console.log(`Is proxy for sudo: ${isProxySudo}`);
    return { sudoKey, isSudo, isProxySudo };
}

// Main runtime upgrade logic
async function performRuntimeUpgrade(api, sudoAccount, wasmPath) {
    // Validate and load WASM
    const absWasmPath = path.resolve(wasmPath);
    if (!fs.existsSync(absWasmPath)) {
        throw new Error('WASM file does not exist: ' + absWasmPath);
    }
    let wasm;
    try {
        wasm = fs.readFileSync(absWasmPath);
    } catch (e) {
        throw new Error('Failed to read WASM file: ' + e.message);
    }
    console.log('Loaded WASM file:', absWasmPath, 'size:', wasm.length);

    // Sudo/proxy detection
    const { sudoKey, isProxySudo } = await getSudoProxyStatus(api, sudoAccount);

    // Step 1: Authorize upgrade
    const codeHash = blake2AsHex(wasm);
    const codeHashU8a = hexToU8a(codeHash);
    const authorizeCall = api.tx.system.authorizeUpgrade(codeHashU8a);
    console.log('authorizeCall method hex:', authorizeCall.method.toHex());
    const sudoAuthorize = api.tx.sudo.sudo(authorizeCall);
    console.log('sudoAuthorize method hex:', sudoAuthorize.method.toHex());
    let extrinsicToSend;
    if (isProxySudo) {
        console.log('Wrapping sudo.sudo(authorizeUpgrade) in proxy.proxy for submission...');
        extrinsicToSend = api.tx.proxy.proxy(sudoKey, null, sudoAuthorize);
        console.log('proxy+sudoAuthorize method hex:', extrinsicToSend.method.toHex());
    } else {
        extrinsicToSend = sudoAuthorize;
    }
    const authorizeSuccess = await sendWithRetry(api, extrinsicToSend, sudoAccount, 'AuthorizeUpgrade');
    if (!authorizeSuccess) {
        throw new Error('Failed to authorize upgrade after retries.');
    }
    console.log('Step 1: AuthorizeUpgrade succeeded.');

    // Step 2: Apply authorized upgrade
    const applyCall = api.tx.system.applyAuthorizedUpgrade('0x' + wasm.toString('hex'));
    console.log('Submitting applyAuthorizedUpgrade (unsigned)...');
    await new Promise(async (resolve, reject) => {
        const unsub = await applyCall.send(({ status }) => {
            if (status.isInBlock) {
                console.log('applyAuthorizedUpgrade included at blockHash', status.asInBlock.toHex());
            } else if (status.isFinalized) {
                console.log('applyAuthorizedUpgrade finalized in block:', status.asFinalized.toHex());
            }
            if (status.isFinalized) {
                unsub();
                resolve();
            }
        }).catch(reject);
    });
    console.log('Step 2: ApplyAuthorizedUpgrade succeeded.');

    // Move the chain forward to apply the upgrade
    console.log('Moving the chain forward by 2 blocks to apply the upgrade...');
    await api.rpc('dev_newBlock', { count: 2 });
    console.log('Moved 2 blocks. Upgrade should now be applied.');
}

// Entrypoint
async function main() {
    //const wsProvider = new WsProvider('wss://349-peaq-network-node.cisys.xyz');
    const wsUrl = process.argv[3];
    const wsProvider = new WsProvider(wsUrl);
    const api = await ApiPromise.create({ provider: wsProvider, noInitWarn: true });
    const SUDO_SEED = process.env.SUDO_SEED || '//Alice';
    const keyring = new Keyring({ type: 'sr25519' });
    const sudoAccount = keyring.addFromUri(SUDO_SEED);

    try {
        // Minimal sudo test mode
        if (process.argv[2] === 'test-sudo') {
            await testSudoRemark(api, sudoAccount);
            await api.disconnect();
            process.exit(0);
        }

        // Normal runtime upgrade mode
        const wasmPath = process.argv[2];
        if (!wasmPath) {
            throw new Error('Usage: node runtime-upgrade.js <path-to-wasm>');
        }
        await performRuntimeUpgrade(api, sudoAccount, wasmPath);
        await api.disconnect();
        console.log('Runtime upgrade process completed.');
    } catch (err) {
        console.error('Fatal error:', err);
        await api.disconnect();
        process.exit(1);
    }
}

main(); 