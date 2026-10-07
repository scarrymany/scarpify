<span class="bars" aria-hidden="true">
  <span></span><span></span><span></span>
</span>

<style>
  .bars {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    width: 14px;
    height: 14px;
  }

  span span {
    flex: 1;
    height: 100%;
    border-radius: 1px;
    background: var(--accent);
    transform-origin: bottom;
    transform: scaleY(0.5);
  }

  @media (prefers-reduced-motion: no-preference) {
    :global(:root:not([data-motion="off"])) span span {
      animation: bounce 0.9s var(--ease) infinite alternate;
    }

    :global(:root:not([data-motion="off"])) span span:nth-child(2) {
      animation-delay: -0.35s;
    }

    :global(:root:not([data-motion="off"])) span span:nth-child(3) {
      animation-delay: -0.6s;
    }
  }

  @keyframes bounce {
    from {
      transform: scaleY(0.25);
    }
    to {
      transform: scaleY(1);
    }
  }
</style>
