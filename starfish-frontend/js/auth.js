document.addEventListener('DOMContentLoaded', function() {
    // Check if already logged in
    const token = localStorage.getItem('starfish_token');
    if (token) {
        validateAndRedirect(token);
    }

    // Handle login form submission
    const loginForm = document.getElementById('login-form');
    loginForm.addEventListener('submit', handleLogin);
});

async function validateAndRedirect(token) {
    try {
        const response = await fetch('/api/auth/validate', {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${token}`
            }
        });

        const data = await response.json();
        if (data.valid) {
            window.location.href = '/dashboard.html';
        } else {
            localStorage.removeItem('starfish_token');
        }
    } catch (error) {
        console.error('Validation error:', error);
        localStorage.removeItem('starfish_token');
    }
}

async function handleLogin(event) {
    event.preventDefault();

    const secretKey = document.getElementById('secret-key').value;
    const loginBtn = document.getElementById('login-btn');
    const btnText = loginBtn.querySelector('.btn-text');
    const btnLoading = loginBtn.querySelector('.btn-loading');
    const errorMessage = document.getElementById('error-message');

    // Reset error state
    errorMessage.style.display = 'none';

    // Show loading state
    loginBtn.disabled = true;
    btnText.style.display = 'none';
    btnLoading.style.display = 'inline';

    try {
        const response = await fetch('/api/auth/login', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ secret_key: secretKey })
        });

        const data = await response.json();

        if (data.success && data.token) {
            localStorage.setItem('starfish_token', data.token);
            window.location.href = '/dashboard.html';
        } else {
            showError(data.error || 'Login failed. Please check your secret key.');
        }
    } catch (error) {
        console.error('Login error:', error);
        showError('Connection error. Please try again.');
    } finally {
        // Reset button state
        loginBtn.disabled = false;
        btnText.style.display = 'inline';
        btnLoading.style.display = 'none';
    }
}

function showError(message) {
    const errorMessage = document.getElementById('error-message');
    errorMessage.textContent = message;
    errorMessage.style.display = 'block';
}
