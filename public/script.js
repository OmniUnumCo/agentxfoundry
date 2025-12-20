// ==========================================
// Mobile Menu Toggle
// ==========================================
const hamburger = document.getElementById('hamburger');
const navMenu = document.getElementById('navMenu');

hamburger.addEventListener('click', () => {
    navMenu.classList.toggle('active');

    // Animate hamburger
    const spans = hamburger.querySelectorAll('span');
    spans[0].style.transform = navMenu.classList.contains('active')
        ? 'rotate(45deg) translate(7px, 7px)'
        : 'none';
    spans[1].style.opacity = navMenu.classList.contains('active') ? '0' : '1';
    spans[2].style.transform = navMenu.classList.contains('active')
        ? 'rotate(-45deg) translate(7px, -7px)'
        : 'none';
});

// Close menu when clicking on a link
document.querySelectorAll('.nav-link').forEach(link => {
    link.addEventListener('click', () => {
        navMenu.classList.remove('active');
        const spans = hamburger.querySelectorAll('span');
        spans[0].style.transform = 'none';
        spans[1].style.opacity = '1';
        spans[2].style.transform = 'none';
    });
});

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

window.addEventListener('scroll', () => {
    if (window.scrollY > 50) {
        navbar.classList.add('scrolled');
    } else {
        navbar.classList.remove('scrolled');
    }
});

// ==========================================
// Contact Form Handling
// ==========================================
const contactForm = document.getElementById('contactForm');

contactForm.addEventListener('submit', (e) => {
    e.preventDefault();

    // Get form data
    const formData = {
        name: document.getElementById('name').value,
        email: document.getElementById('email').value,
        phone: document.getElementById('phone').value,
        service: document.getElementById('service').value,
        message: document.getElementById('message').value
    };

    // Here you would typically send the data to a server
    // For now, we'll just show a success message

    // Create success message
    const successMessage = document.createElement('div');
    successMessage.style.cssText = `
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%);
        color: white;
        padding: 2rem 3rem;
        border-radius: 15px;
        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3);
        z-index: 10000;
        text-align: center;
        animation: fadeIn 0.3s ease;
    `;

    successMessage.innerHTML = `
        <h3 style="margin-bottom: 1rem; font-size: 1.5rem;">Thank You!</h3>
        <p style="margin-bottom: 1.5rem;">We've received your request and will get back to you within 24 hours.</p>
        <button onclick="this.parentElement.remove()" style="
            background: white;
            color: #1e40af;
            border: none;
            padding: 0.75rem 2rem;
            border-radius: 25px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
        " onmouseover="this.style.transform='scale(1.05)'" onmouseout="this.style.transform='scale(1)'">
            Close
        </button>
    `;

    document.body.appendChild(successMessage);

    // Reset form
    contactForm.reset();

    // Auto remove message after 5 seconds
    setTimeout(() => {
        if (successMessage.parentElement) {
            successMessage.remove();
        }
    }, 5000);

    // Log form data (in production, you'd send this to a server)
    console.log('Form submitted:', formData);
});

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
    const currentYear = new Date().getFullYear();

    // Prefer a dedicated year element if present
    const yearElement = document.querySelector('#footer-year, [data-footer-year]');
    if (yearElement) {
        yearElement.textContent = currentYear;
        return;
    }

    // Fallback: update the first year-like token in the footer text
    const footerText = document.querySelector('.footer-bottom p');
    if (footerText) {
        const yearPattern = /\b20\d{2}\b/;
        const updatedHtml = footerText.innerHTML.replace(yearPattern, String(currentYear));
        footerText.innerHTML = updatedHtml;
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
document.querySelectorAll('.addon-item').forEach(item => {
    item.addEventListener('click', function(event) {
        // Create a ripple effect
        const ripple = document.createElement('div');
        
        // Get click position relative to the element
        const rect = this.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        ripple.style.cssText = `
            position: absolute;
            border-radius: 50%;
            background: rgba(255, 255, 255, 0.6);
            width: 100px;
            height: 100px;
            left: ${x}px;
            top: ${y}px;
            margin-top: -50px;
            margin-left: -50px;
            animation: ripple 0.6s;
            pointer-events: none;
        `;

        this.style.position = 'relative';
        this.appendChild(ripple);

        setTimeout(() => ripple.remove(), 600);
    });
});

// Add ripple animation to CSS dynamically
const style = document.createElement('style');
style.textContent = `
    @keyframes ripple {
        from {
            opacity: 1;
            transform: scale(0);
        }
        to {
            opacity: 0;
            transform: scale(2);
        }
    }
`;
document.head.appendChild(style);

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
    indicator.style.cssText = `
        position: fixed;
        top: 0;
        left: 0;
        height: 3px;
        background: linear-gradient(90deg, #1e40af, #fbbf24);
        z-index: 10000;
        transition: width 0.1s ease;
    `;
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
