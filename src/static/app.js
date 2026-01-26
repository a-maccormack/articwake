(function() {
    let token = null;
    let pollInterval = null;
    let pendingUnlockPrompt = false;

    // Server states
    const STATE = {
        OFFLINE: 'offline',
        WAKING: 'waking',
        INITRD: 'initrd',
        UNLOCKING: 'unlocking',
        BOOTING: 'booting',
        READY: 'ready'
    };

    let currentState = STATE.OFFLINE;

    const $ = id => document.getElementById(id);

    function showMessage(text, isError = false) {
        const el = $('message');
        el.textContent = text;
        el.className = 'message ' + (isError ? 'error' : 'success');
        el.classList.remove('hidden');
        setTimeout(() => el.classList.add('hidden'), 5000);
    }

    async function api(endpoint, method = 'GET', body = null) {
        const headers = { 'Content-Type': 'application/json' };
        if (token) headers['Authorization'] = 'Bearer ' + token;

        const opts = { method, headers };
        if (body) opts.body = JSON.stringify(body);

        const res = await fetch('/api/' + endpoint, opts);
        const data = await res.json();

        if (!res.ok) {
            throw new Error(data.error || 'Request failed');
        }
        return data;
    }

    // Determine state from status data
    function determineState(status, prevState) {
        if (status.system_ssh_open) {
            return STATE.READY;
        }
        if (status.initrd_ssh_open) {
            if (prevState === STATE.UNLOCKING || prevState === STATE.BOOTING) {
                return STATE.BOOTING;
            }
            return STATE.INITRD;
        }
        if (status.reachable) {
            if (prevState === STATE.WAKING) {
                return STATE.WAKING; // Still waking, waiting for initrd
            }
            return STATE.OFFLINE; // Reachable but no SSH = weird state, treat as offline
        }
        if (prevState === STATE.WAKING) {
            return STATE.WAKING;
        }
        return STATE.OFFLINE;
    }

    function updateUI(state, status) {
        const wakeCircle = $('step-wake-circle');
        const wakeLine = $('step-wake-line');
        const wakeDesc = $('step-wake-desc');
        const wakeAction = $('step-wake-action');
        const wolBtn = $('wol-btn');

        const unlockCircle = $('step-unlock-circle');
        const unlockLine = $('step-unlock-line');
        const unlockDesc = $('step-unlock-desc');
        const unlockAction = $('step-unlock-action');

        const readyCircle = $('step-ready-circle');
        const readyDesc = $('step-ready-desc');

        const onlineBadge = $('online-badge');

        // Reset all
        wakeCircle.className = 'step-circle';
        wakeCircle.innerHTML = '1';
        wakeLine.className = 'step-line';
        unlockCircle.className = 'step-circle';
        unlockCircle.innerHTML = '2';
        unlockLine.className = 'step-line';
        readyCircle.className = 'step-circle';
        readyCircle.innerHTML = '3';
        onlineBadge.classList.add('hidden');
        wakeAction.classList.remove('hidden');
        unlockAction.classList.add('hidden');
        wolBtn.disabled = false;

        switch (state) {
            case STATE.OFFLINE:
                wakeCircle.classList.add('active');
                wakeDesc.textContent = 'Server is offline';
                unlockDesc.textContent = 'Waiting for server to wake';
                readyDesc.textContent = 'Waiting for boot';
                break;

            case STATE.WAKING:
                wakeCircle.classList.add('active');
                wakeCircle.innerHTML = '<div class="spinner-small"></div>';
                wakeDesc.textContent = 'Sending wake signal...';
                wakeAction.classList.add('hidden');
                unlockDesc.textContent = 'Waiting for server to wake';
                readyDesc.textContent = 'Waiting for boot';
                break;

            case STATE.INITRD:
                wakeCircle.classList.add('completed');
                wakeCircle.innerHTML = '&#10003;';
                wakeLine.classList.add('completed');
                wakeDesc.textContent = 'Server is awake';
                wakeAction.classList.add('hidden');

                unlockCircle.classList.add('active');
                unlockDesc.textContent = 'Initrd SSH ready - enter passphrase';
                unlockAction.classList.remove('hidden');

                readyDesc.textContent = 'Waiting for unlock';
                break;

            case STATE.UNLOCKING:
            case STATE.BOOTING:
                wakeCircle.classList.add('completed');
                wakeCircle.innerHTML = '&#10003;';
                wakeLine.classList.add('completed');
                wakeDesc.textContent = 'Server is awake';
                wakeAction.classList.add('hidden');

                unlockCircle.classList.add('active');
                unlockCircle.innerHTML = '<div class="spinner-small"></div>';
                unlockLine.classList.add('completed');
                unlockDesc.textContent = 'Passphrase sent, booting...';
                unlockAction.classList.add('hidden');

                readyCircle.classList.add('active');
                readyDesc.textContent = 'Waiting for system SSH...';
                break;

            case STATE.READY:
                wakeCircle.classList.add('completed');
                wakeCircle.innerHTML = '&#10003;';
                wakeLine.classList.add('completed');
                wakeDesc.textContent = 'Server is awake';
                wakeAction.classList.add('hidden');

                unlockCircle.classList.add('completed');
                unlockCircle.innerHTML = '&#10003;';
                unlockLine.classList.add('completed');
                unlockDesc.textContent = 'Disk unlocked';
                unlockAction.classList.add('hidden');

                readyCircle.classList.add('completed');
                readyCircle.innerHTML = '&#10003;';
                readyDesc.textContent = 'System is ready!';

                onlineBadge.classList.remove('hidden');
                break;
        }
    }

    async function refreshStatus() {
        try {
            const status = await api('status');
            const newState = determineState(status, currentState);

            // Auto-show unlock modal when initrd becomes available
            if (newState === STATE.INITRD && currentState !== STATE.INITRD && !pendingUnlockPrompt) {
                pendingUnlockPrompt = true;
                setTimeout(() => {
                    showUnlockModal();
                    pendingUnlockPrompt = false;
                }, 500);
            }

            // Notify when system becomes ready
            if (newState === STATE.READY && currentState !== STATE.READY) {
                showMessage('System is fully booted and ready!');
                stopPolling();
            }

            currentState = newState;
            updateUI(currentState, status);
            return status;
        } catch (e) {
            if (e.message.includes('token') || e.message.includes('Unauthorized')) {
                logout();
            }
            throw e;
        }
    }

    function startPolling(interval = 2000) {
        stopPolling();
        pollInterval = setInterval(async () => {
            try {
                await refreshStatus();
            } catch (e) {
                console.error('Poll failed:', e);
            }
        }, interval);
    }

    function stopPolling() {
        if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
        }
    }

    async function checkBackendReachable() {
        try {
            const res = await fetch('/api/status', { method: 'GET' });
            return res.status === 401; // 401 means backend is up but needs auth
        } catch {
            return false;
        }
    }

    async function init() {
        // Show loading, check if backend is reachable
        $('loading-section').classList.remove('hidden');
        $('auth-section').classList.add('hidden');
        $('main-section').classList.add('hidden');

        const reachable = await checkBackendReachable();

        $('loading-section').classList.add('hidden');

        if (reachable) {
            $('auth-section').classList.remove('hidden');
            $('pin-input').focus();
        } else {
            showMessage('Cannot connect to server', true);
            $('auth-section').classList.remove('hidden');
        }
    }

    async function authenticate() {
        const pin = $('pin-input').value;
        if (!pin) {
            showMessage('Please enter a PIN', true);
            return;
        }

        try {
            $('auth-btn').disabled = true;
            const data = await api('auth', 'POST', { pin });
            token = data.token;
            $('auth-section').classList.add('hidden');
            $('pin-input').value = '';

            // Show loading while fetching initial status
            $('loading-section').classList.remove('hidden');

            await refreshStatus();

            $('loading-section').classList.add('hidden');
            $('main-section').classList.remove('hidden');

            // Start slow polling for background updates
            startPolling(10000);
        } catch (e) {
            showMessage(e.message, true);
        } finally {
            $('auth-btn').disabled = false;
        }
    }

    async function sendWol() {
        try {
            $('wol-btn').disabled = true;
            currentState = STATE.WAKING;
            updateUI(currentState, null);

            await api('wol', 'POST');

            // Start fast polling to detect when initrd comes up
            startPolling(2000);

            // Timeout after 2 minutes
            setTimeout(() => {
                if (currentState === STATE.WAKING) {
                    showMessage('Server did not respond within 2 minutes', true);
                    currentState = STATE.OFFLINE;
                    updateUI(currentState, null);
                    stopPolling();
                    startPolling(10000);
                }
            }, 120000);
        } catch (e) {
            showMessage(e.message, true);
            currentState = STATE.OFFLINE;
            updateUI(currentState, null);
        }
    }

    function showUnlockModal() {
        $('unlock-modal').classList.add('active');
        $('passphrase-input').focus();
    }

    function hideUnlockModal() {
        $('unlock-modal').classList.remove('active');
        $('passphrase-input').value = '';
    }

    async function submitUnlock() {
        const passphrase = $('passphrase-input').value;
        if (!passphrase) {
            showMessage('Please enter a passphrase', true);
            return;
        }

        try {
            $('submit-unlock-btn').disabled = true;
            await api('unlock', 'POST', { passphrase });
            hideUnlockModal();

            currentState = STATE.BOOTING;
            updateUI(currentState, null);

            // Start fast polling to detect when system SSH comes up
            startPolling(2000);

            // Timeout after 3 minutes
            setTimeout(() => {
                if (currentState === STATE.BOOTING || currentState === STATE.UNLOCKING) {
                    showMessage('System did not fully boot within 3 minutes', true);
                    stopPolling();
                    startPolling(10000);
                }
            }, 180000);
        } catch (e) {
            showMessage(e.message, true);
        } finally {
            $('submit-unlock-btn').disabled = false;
        }
    }

    function logout() {
        token = null;
        stopPolling();
        currentState = STATE.OFFLINE;
        $('auth-section').classList.remove('hidden');
        $('main-section').classList.add('hidden');
        $('online-badge').classList.add('hidden');
    }

    // Event listeners
    $('auth-btn').addEventListener('click', authenticate);
    $('pin-input').addEventListener('keypress', e => {
        if (e.key === 'Enter') authenticate();
    });

    $('wol-btn').addEventListener('click', sendWol);
    $('unlock-btn').addEventListener('click', showUnlockModal);
    $('refresh-btn').addEventListener('click', refreshStatus);

    $('submit-unlock-btn').addEventListener('click', submitUnlock);
    $('cancel-unlock-btn').addEventListener('click', hideUnlockModal);
    $('passphrase-input').addEventListener('keypress', e => {
        if (e.key === 'Enter') submitUnlock();
    });

    $('unlock-modal').addEventListener('click', e => {
        if (e.target === $('unlock-modal')) hideUnlockModal();
    });

    // Initialize
    init();
})();
