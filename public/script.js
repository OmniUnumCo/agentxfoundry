// ==========================================
// Mobile Menu Toggle
// ==========================================
const hamburger = document.getElementById('hamburger');
const navMenu = document.getElementById('navMenu');

if (hamburger && navMenu) {
    hamburger.addEventListener('click', () => {
        const isActive = navMenu.classList.toggle('active');
        
        // Update aria-expanded attribute
        hamburger.setAttribute('aria-expanded', isActive);

        // Animate hamburger
        const spans = hamburger.querySelectorAll('span');
        spans[0].style.transform = isActive
            ? 'rotate(45deg) translate(7px, 7px)'
            : 'none';
        spans[1].style.opacity = isActive ? '0' : '1';
        spans[2].style.transform = isActive
            ? 'rotate(-45deg) translate(7px, -7px)'
            : 'none';
    });

    // Close menu when clicking on a link
    document.querySelectorAll('.nav-link').forEach(link => {
        link.addEventListener('click', () => {
            navMenu.classList.remove('active');
            hamburger.setAttribute('aria-expanded', 'false');
            const spans = hamburger.querySelectorAll('span');
            spans[0].style.transform = 'none';
            spans[1].style.opacity = '1';
            spans[2].style.transform = 'none';
        });
    });
}

// ==========================================
// Smooth Scrolling
// ==========================================
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            const headerOffset = 80;
            const elementPosition = target.getBoundingClientRect().top;
            const offsetPosition = elementPosition + window.pageYOffset - headerOffset;

            window.scrollTo({
                top: offsetPosition,
                behavior: 'smooth'
            });
        }
    });
});

// ==========================================
// Navbar Background on Scroll
// ==========================================
const navbar = document.getElementById('navbar');

if (navbar) {
    window.addEventListener('scroll', () => {
        if (window.scrollY > 50) {
            navbar.classList.add('scrolled');
        } else {
            navbar.classList.remove('scrolled');
        }
    });
}

// ==========================================
// Contact Form Handling
// ==========================================
const contactForm = document.getElementById('contactForm');

if (contactForm) {
    // Simple email validation pattern for basic format checking
    // Validates: local-part@domain.tld format
    // Limitations: Does not validate international domains, special characters, or all RFC 5322 rules
    // For production: Consider server-side validation for comprehensive checking
    const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    
    function isValidEmail(email) {
        return EMAIL_PATTERN.test(email);
    }

    contactForm.addEventListener('submit', (e) => {
        e.preventDefault();

        const emailInput = document.getElementById('email');
        const emailValue = emailInput.value.trim();

        // Validate email format
        if (!isValidEmail(emailValue)) {
            if (typeof emailInput.setCustomValidity === 'function') {
                emailInput.setCustomValidity('Please enter a valid email address.');
                emailInput.reportValidity();
            }
            emailInput.focus();
            return;
        } else if (typeof emailInput.setCustomValidity === 'function') {
            emailInput.setCustomValidity('');
        }

        // Get form data
        const formData = {
            name: document.getElementById('name').value,
            email: emailValue,
            phone: document.getElementById('phone').value,
            service: document.getElementById('service').value,
            message: document.getElementById('message').value
        };

        // NOTE: This form is for demonstration purposes only
        // No data is actually sent to a server
        // In production, integrate with a backend service or email API

        // Create success message
        const successMessage = document.createElement('div');
        successMessage.className = 'form-success-modal';

        successMessage.innerHTML = `
            <h3 class="modal-title">Form Submitted!</h3>
            <p class="modal-message">Your information has been received.</p>
            <p class="modal-note">
                <strong>Note:</strong> This is a demo form. No data is sent to a server.<br>
                In production, integrate with a backend service.
            </p>
            <button class="close-modal-btn">Close</button>
        `;

        document.body.appendChild(successMessage);

        // Add event listener to close button
        const closeBtn = successMessage.querySelector('.close-modal-btn');
        if (closeBtn) {
            closeBtn.addEventListener('click', () => {
                successMessage.remove();
            });
        }

        // Reset form
        contactForm.reset();

        // Auto remove message after 5 seconds
        setTimeout(() => {
            if (successMessage.parentElement) {
                successMessage.remove();
            }
        }, 5000);

        // Log form data for development purposes
        console.log('Form submitted:', formData);
    });
}

// ==========================================
// Intersection Observer for Fade-in Animations
// ==========================================
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.style.opacity = '1';
            entry.target.style.transform = 'translateY(0)';
        }
    });
}, observerOptions);

// Observe elements for animation
document.addEventListener('DOMContentLoaded', () => {
    const animatedElements = document.querySelectorAll(
        '.feature-card, .service-card, .gallery-item, .info-item'
    );

    animatedElements.forEach(el => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(30px)';
        el.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(el);
    });
});

// ==========================================
// Dynamic Year in Footer
// ==========================================
document.addEventListener('DOMContentLoaded', () => {
    const yearElement = document.getElementById('copyright-year');
    if (yearElement) {
        yearElement.textContent = new Date().getFullYear();
    }
});

// ==========================================
// Service Card Hover Effect Enhancement
// ==========================================
document.querySelectorAll('.service-card').forEach(card => {
    card.addEventListener('mouseenter', function() {
        this.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)';
    });
});

// ==========================================
// Add-on Items Interactive Behavior
// ==========================================
// Ripple effect constants (must match CSS .ripple-effect dimensions)
const RIPPLE_SIZE = 100; // Width and height in pixels
const RIPPLE_HALF = RIPPLE_SIZE / 2;

document.querySelectorAll('.addon-item').forEach(item => {
    item.addEventListener('click', function(e) {
        // Create a ripple effect at click position
        const ripple = document.createElement('div');
        const rect = this.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        ripple.className = 'ripple-effect';
        // Center the ripple at click coordinates (offset by half the ripple size)
        ripple.style.left = `${x - RIPPLE_HALF}px`;
        ripple.style.top = `${y - RIPPLE_HALF}px`;

        // Add ripple-container class if not already present
        if (!this.classList.contains('ripple-container')) {
            this.classList.add('ripple-container');
        }
        this.appendChild(ripple);

        setTimeout(() => ripple.remove(), 600);
    });
});

// ==========================================
// Gallery Item Click Handler
// ==========================================
document.querySelectorAll('.gallery-item').forEach(item => {
    item.addEventListener('click', function() {
        // In a real implementation, this would open a lightbox
        console.log('Gallery item clicked');

        // Add a visual feedback
        const originalTransform = this.style.transform;
        this.style.transform = 'scale(0.95)';
        setTimeout(() => {
            this.style.transform = originalTransform;
        }, 200);
    });
});

// ==========================================
// Scroll Progress Indicator
// ==========================================
const createScrollIndicator = () => {
    const indicator = document.createElement('div');
    indicator.className = 'scroll-progress-indicator';
    document.body.appendChild(indicator);

    window.addEventListener('scroll', () => {
        const windowHeight = document.documentElement.scrollHeight - window.innerHeight;
        const scrolled = (window.scrollY / windowHeight) * 100;
        indicator.style.width = scrolled + '%';
    });
};

createScrollIndicator();

// ==========================================
// Form Input Validation & Styling
// ==========================================
const inputs = document.querySelectorAll('input, select, textarea');

inputs.forEach(input => {
    input.addEventListener('focus', function() {
        this.parentElement.style.transform = 'scale(1.02)';
        this.parentElement.style.transition = 'transform 0.2s ease';
    });

    input.addEventListener('blur', function() {
        this.parentElement.style.transform = 'scale(1)';

        // Validation feedback
        if (this.hasAttribute('required') && !this.value) {
            this.style.borderColor = '#ef4444';
        } else if (this.value) {
            this.style.borderColor = '#10b981';
        }
    });
});

// ==========================================
// Lazy Loading Placeholder Enhancement
// ==========================================
const placeholders = document.querySelectorAll('.gallery-placeholder, .image-placeholder');

placeholders.forEach((placeholder, index) => {
    setTimeout(() => {
        placeholder.style.animation = 'pulse 2s ease-in-out infinite';
    }, index * 100);
});

// ==========================================
// Console Welcome Message
// ==========================================
console.log(
    '%c🚗 MMJ Detailing Website',
    'font-size: 20px; font-weight: bold; color: #1e40af;'
);
console.log(
    '%cPremium Mobile Auto Detailing in Carver, MA',
    'font-size: 14px; color: #64748b;'
);
console.log(
    '%cWebsite designed for optimal user experience',
    'font-size: 12px; color: #94a3b8;'
);
