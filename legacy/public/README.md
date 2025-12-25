# MMJ Detailing Website

Premium mobile auto detailing website for MMJ Detailing, based in Carver, MA.

## Features

- **Modern, Responsive Design**: Beautiful UI that works seamlessly on all devices
- **Interactive Elements**: Smooth scrolling, animations, and user feedback
- **Service Packages**: Three tier packages (Express, Premium, Signature) with clear pricing
- **Contact Form**: Easy-to-use form for quote requests
- **Mobile-First**: Optimized for mobile users with hamburger menu
- **Performance**: Fast loading with optimized CSS and JavaScript

## Structure

```
public/
├── index.html      # Main HTML file
├── styles.css      # All styling and responsive design
├── script.js       # Interactive functionality
└── assets/         # Images and media (to be added)
    └── images/
```

## Sections

1. **Hero Section**: Eye-catching introduction with CTA buttons
2. **Features**: Key benefits (Mobile Service, Premium Quality, etc.)
3. **Services**: Three detailed service packages with pricing
4. **About**: Company information and statistics
5. **Gallery**: Showcase of work (placeholders for images)
6. **Contact**: Contact form and business information
7. **Footer**: Links and additional information

## Technology Stack

- Pure HTML5
- CSS3 with custom properties and modern layouts
- Vanilla JavaScript (no dependencies)
- Google Fonts (Montserrat & Poppins)

## Customization

### Colors
Edit CSS variables in `styles.css`:
```css
:root {
    --primary-color: #1e40af;
    --accent-color: #fbbf24;
    /* ... more variables */
}
```

### Content
All content can be edited directly in `index.html`

### Images
Add your images to `public/assets/images/` and update the placeholders

## Browser Support

- Chrome (latest)
- Firefox (latest)
- Safari (latest)
- Edge (latest)
- Mobile browsers

## Setup

Simply open `index.html` in a web browser or serve it with any web server:

```bash
# Using Python
python -m http.server 8000

# Using Node.js
npx serve

# Using PHP
php -S localhost:8000
```

Then visit `http://localhost:8000`

## Future Enhancements

- Add real images to gallery
- Integrate with booking system
- Add customer testimonials
- Connect contact form to email service
- Add Google Maps integration
- Implement image lightbox for gallery
- Add before/after image sliders

## License

All rights reserved - MMJ Detailing 2024
