const navItems = document.querySelectorAll('.nav-item');
const sections = document.querySelectorAll('.section');

function navigate(sectionId) {
  navItems.forEach((item) => {
    const isActive = item.dataset.section === sectionId;
    item.classList.toggle('active', isActive);
    item.setAttribute('aria-current', isActive ? 'page' : 'false');
  });
  sections.forEach((section) => {
    section.classList.toggle('active', section.id === sectionId);
  });
}

navItems.forEach((item) => {
  item.addEventListener('click', () => navigate(item.dataset.section));
});

// "View all" links inside panels
document.querySelectorAll('[data-nav]').forEach((btn) => {
  btn.addEventListener('click', () => navigate(btn.dataset.nav));
});
