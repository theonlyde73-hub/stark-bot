document.addEventListener('DOMContentLoaded', function() {
    const token = localStorage.getItem('starfish_token');

    if (!token) {
        redirectToLogin();
        return;
    }

    // Validate token and load dashboard
    loadDashboard(token);

    // Handle logout
    document.getElementById('logout-btn').addEventListener('click', () => handleLogout(token));
});

function redirectToLogin() {
    window.location.href = '/';
}

async function loadDashboard(token) {
    const loadingEl = document.getElementById('loading');
    const dashboardData = document.getElementById('dashboard-data');
    const errorMessage = document.getElementById('error-message');

    try {
        const response = await fetch('/api/dashboard', {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${token}`
            }
        });

        if (response.status === 401) {
            localStorage.removeItem('starfish_token');
            redirectToLogin();
            return;
        }

        const data = await response.json();

        if (data.error) {
            showError(data.error);
            return;
        }

        // Display dashboard data
        document.getElementById('welcome-text').textContent = data.message;
        document.getElementById('timestamp').textContent = `Last updated: ${formatTimestamp(data.timestamp)}`;

        loadingEl.style.display = 'none';
        dashboardData.style.display = 'block';

    } catch (error) {
        console.error('Dashboard error:', error);
        showError('Failed to load dashboard. Please try again.');
    }
}

async function handleLogout(token) {
    try {
        await fetch('/api/auth/logout', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ token: token })
        });
    } catch (error) {
        console.error('Logout error:', error);
    } finally {
        localStorage.removeItem('starfish_token');
        redirectToLogin();
    }
}

function showError(message) {
    const loadingEl = document.getElementById('loading');
    const errorMessage = document.getElementById('error-message');

    loadingEl.style.display = 'none';
    errorMessage.textContent = message;
    errorMessage.style.display = 'block';
}

function formatTimestamp(isoString) {
    const date = new Date(isoString);
    return date.toLocaleString();
}
