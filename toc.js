// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><a href="why-debtmap.html">Why Debtmap?</a></li><li class="chapter-item expanded affix "><li class="part-title">User Guide</li><li class="chapter-item expanded "><a href="getting-started.html"><strong aria-hidden="true">1.</strong> Getting Started</a></li><li class="chapter-item expanded "><a href="cli-reference.html"><strong aria-hidden="true">2.</strong> CLI Reference</a></li><li class="chapter-item expanded "><a href="analysis-guide.html"><strong aria-hidden="true">3.</strong> Analysis Guide</a></li><li class="chapter-item expanded "><a href="configuration.html"><strong aria-hidden="true">4.</strong> Configuration</a></li><li class="chapter-item expanded "><a href="suppression-patterns.html"><strong aria-hidden="true">5.</strong> Suppression Patterns</a></li><li class="chapter-item expanded "><a href="output-formats.html"><strong aria-hidden="true">6.</strong> Output Formats</a></li><li class="chapter-item expanded affix "><li class="part-title">Advanced Topics</li><li class="chapter-item expanded "><a href="architecture.html"><strong aria-hidden="true">7.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="cache-management.html"><strong aria-hidden="true">8.</strong> Cache Management</a></li><li class="chapter-item expanded "><a href="context-providers.html"><strong aria-hidden="true">9.</strong> Context Providers</a></li><li class="chapter-item expanded "><a href="coverage-analysis.html"><strong aria-hidden="true">10.</strong> Coverage Analysis</a></li><li class="chapter-item expanded "><a href="coverage-integration.html"><strong aria-hidden="true">11.</strong> Coverage Integration</a></li><li class="chapter-item expanded "><a href="entropy-analysis.html"><strong aria-hidden="true">12.</strong> Entropy Analysis</a></li><li class="chapter-item expanded "><a href="god-object-detection.html"><strong aria-hidden="true">13.</strong> God Object Detection</a></li><li class="chapter-item expanded "><a href="parallel-processing.html"><strong aria-hidden="true">14.</strong> Parallel Processing</a></li><li class="chapter-item expanded "><a href="prodigy-integration.html"><strong aria-hidden="true">15.</strong> Prodigy Integration</a></li><li class="chapter-item expanded "><a href="responsibility-analysis.html"><strong aria-hidden="true">16.</strong> Responsibility Analysis</a></li><li class="chapter-item expanded "><a href="scoring-strategies.html"><strong aria-hidden="true">17.</strong> Scoring Strategies</a></li><li class="chapter-item expanded "><a href="tiered-prioritization.html"><strong aria-hidden="true">18.</strong> Tiered Prioritization</a></li><li class="chapter-item expanded affix "><li class="part-title">Reference</li><li class="chapter-item expanded "><a href="metrics-reference.html"><strong aria-hidden="true">19.</strong> Metrics Reference</a></li><li class="chapter-item expanded "><a href="examples.html"><strong aria-hidden="true">20.</strong> Examples</a></li><li class="chapter-item expanded "><a href="faq.html"><strong aria-hidden="true">21.</strong> FAQ</a></li><li class="chapter-item expanded "><a href="troubleshooting.html"><strong aria-hidden="true">22.</strong> Troubleshooting</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
