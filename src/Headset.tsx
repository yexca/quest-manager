import { useId } from 'react';

export function Headset({ model }: { model?: string }) {
  const id = useId();
  const normalized = model?.toLowerCase().replace(/[\s_-]+/g, '').replace(/^(meta|oculus)/, '');
  const variant = normalized === 'quest3s' ? 'quest3s' : normalized === 'quest3' ? 'quest3' : normalized === 'quest2' ? 'quest2' : 'generic';
  const modern = variant === 'quest3' || variant === 'quest3s';
  const paint = (name: string) => `url(#${id}-${name})`;

  return <svg className={`headset${modern ? ' headset-front' : ''}`} viewBox="0 0 440 280" fill="none" aria-hidden="true" data-model={variant}>
    <defs>
      <linearGradient id={`${id}-visor`} x1="135" y1="83" x2="256" y2="241" gradientUnits="userSpaceOnUse"><stop stopColor="#fff" /><stop offset="1" stopColor="#cdd2cc" /></linearGradient>
      <linearGradient id={`${id}-strap`} x1="115" y1="45" x2="302" y2="113" gradientUnits="userSpaceOnUse"><stop stopColor="#7e8b7e" /><stop offset="1" stopColor="#eef1e9" /></linearGradient>
      <linearGradient id={`${id}-face`} x1="185" y1="95" x2="261" y2="234" gradientUnits="userSpaceOnUse"><stop stopColor="#fdfefb" /><stop offset=".58" stopColor="#f0f2ed" /><stop offset="1" stopColor="#e2e7df" /></linearGradient>
      <linearGradient id={`${id}-shell`} x1="220" y1="112" x2="220" y2="246" gradientUnits="userSpaceOnUse"><stop stopColor="#eef1e9" /><stop offset="1" stopColor="#bcc5b9" /></linearGradient>
      <linearGradient id={`${id}-fabric`} x1="192" y1="30" x2="245" y2="112" gradientUnits="userSpaceOnUse"><stop stopColor="#fafbf7" /><stop offset="1" stopColor="#cdd4c8" /></linearGradient>
      <linearGradient id={`${id}-glass`} x1="0" y1="0" x2="0" y2="1"><stop stopColor="#172024" /><stop offset=".55" stopColor="#29343e" /><stop offset="1" stopColor="#56616c" /></linearGradient>
      <radialGradient id={`${id}-lens`}><stop stopColor="#40515d" /><stop offset=".5" stopColor="#23303a" /><stop offset="1" stopColor="#151d23" /></radialGradient>
      <pattern id={`${id}-weave`} width="3" height="3" patternUnits="userSpaceOnUse"><path d="M0 1h3M1 0v3" stroke="#66755e" strokeWidth=".35" opacity=".14" /></pattern>
      <filter id={`${id}-shadow`}><feGaussianBlur stdDeviation="10" /></filter>
    </defs>
    <ellipse cx={modern ? 220 : 231} cy={modern ? 249 : 239} rx={modern ? 127 : 123} ry={modern ? 11 : 16} fill="#152f20" opacity={modern ? .14 : .19} filter={paint('shadow')} />

    {modern ? <>
      {/* The original fabric strap attaches centrally; the rear band sits behind it. */}
      <path d="M139 116c10-37 29-72 66-78 43-7 81 27 96 78" stroke="#b9c3b2" strokeWidth="16" />
      <path d="M139 109c11-34 33-65 67-70 42-6 75 25 92 70" stroke="#e0e5d9" strokeWidth="11" />
      <path d="M197 33q23-5 46 0l6 78h-58Z" fill={paint('fabric')} stroke="#c9d1c2" />
      <path d="M197 33q23-5 46 0l6 78h-58Z" fill={paint('weave')} />
      <path d="m199 36-4 69m45-69 5 69" stroke="#fff" opacity=".6" />
      <path d="M101 117c27-36 206-42 239-2l8 76H93Z" fill="#424b41" />
      <path d="M114 111c33-25 182-29 213-1" stroke="#596251" strokeWidth="5" strokeLinecap="round" />
      <path d="m82 127-17-7c-8 25-9 62-1 87l21-4m269-76 17-7c8 25 9 62 1 87l-21-4" fill={paint('fabric')} stroke="#bfc8b8" strokeWidth="1.5" />
      <path d="M68 136c-4 20-4 41 0 60m300-60c4 20 4 41 0 60" stroke="#fff" strokeWidth="3" opacity=".7" />

      {variant === 'quest3' ? <>
        <path d="M93 210c5 25 23 39 55 42l39-2q10-16 32-16t32 16l39 2c33-3 51-17 57-42Z" fill="#444d43" />
        <path d="M102 227q20 20 80 17m74 0q55 3 80-17" stroke="#626b5b" strokeWidth="4" strokeLinecap="round" />
        <path d="M128 100c42-13 142-13 181-1 34 10 50 33 50 66 0 35-12 58-42 70-42 17-153 17-195 0-30-12-44-35-43-70 0-32 15-55 49-65Z" fill={paint('shell')} stroke="#bec8b8" />
        <path d="M131 103c43-12 136-12 175-1 31 9 47 30 47 60 0 31-13 51-39 62-41 16-147 16-188 0-28-11-41-32-40-63 0-28 14-49 45-58Z" fill={paint('face')} stroke="#aab6a2" strokeWidth="1.2" />
        <path d="M121 111c42-16 150-18 194-1" stroke="#fff" strokeWidth="2" strokeLinecap="round" />
        <path d="M210 118c-4-7-9-3-9 3 0 8 7 7 12-1s11-7 11 1c0 7-5 9-10 1" stroke="#d3dacf" strokeWidth="2.2" strokeLinecap="round" />
        {[163, 219, 275].map((x, index) => <g key={x} transform={`translate(${x} 177)`}>
          <rect x="-14" y="-2" width="28" height="51" rx="14" fill="#fcfdf9" stroke="#ccd4c6" />
          <rect x="-11.5" width="23" height="46" rx="11.5" fill={paint('glass')} stroke="#586250" strokeWidth=".6" />
          {index !== 1 && <>
            <circle cy="13" r="7.1" fill={paint('lens')} stroke="#64727a" strokeWidth=".6" />
            <circle cy="13" r="3.5" stroke="#53636c" strokeWidth=".8" />
            <circle cy="34" r="5.7" fill={paint('lens')} opacity=".7" />
            <circle cx="-2" cy="11" r="1.5" fill="#8a9ba4" opacity=".45" />
          </>}
          <path d="M-8 32v3q0 7 7 8" stroke="#bcc7cd" strokeWidth=".7" opacity=".3" />
        </g>)}
        <path d="m127 236 22 3" stroke="#83917b" strokeWidth="2.5" strokeLinecap="round" />
        <path d="M163 240h2m5 1h2m5 0h2" stroke="#63735b" strokeWidth="1.5" strokeLinecap="round" />
        <ellipse cx="297" cy="237" rx="10" ry="2.5" fill="#a6b29d" />
      </> : <>
        <path d="M123 96c41-13 149-13 190 0 34 11 51 37 50 72-1 35-17 62-47 74-44 17-150 17-194 0-30-12-46-39-47-74-1-35 15-61 48-72Z" fill={paint('shell')} stroke="#b5c0ad" />
        <path d="M125 102c40-12 145-12 186 0 31 10 46 34 45 67-1 31-15 56-43 67-42 17-144 17-186 0-28-11-43-36-44-67-1-33 13-57 42-67Z" fill={paint('face')} stroke="#a9b59f" strokeWidth="1.2" />
        <path d="M113 112c41-19 164-20 210-1" stroke="#fff" strokeWidth="2" strokeLinecap="round" />
        <circle cx="219" cy="115" r="1.2" fill="#6f7c65" />
        {/* Each cluster has two cameras nearer the centre and an outer IR illuminator. */}
        {[{ x: 164, side: -1 }, { x: 274, side: 1 }].map(({ x, side }) => <g key={x} transform={`translate(${x} 193)`}>
          <circle r="10.5" fill="#28312d" stroke="#aab4a1" />
          <circle r="7.5" fill={paint('lens')} stroke="#58696b" strokeWidth=".6" />
          <circle r="4.3" stroke="#6d7b7e" strokeWidth=".7" opacity=".6" />
          <circle cy="23" r="10" fill="#303832" stroke="#aab4a1" />
          <circle cy="23" r="5.5" fill="#111c21" stroke="#657174" strokeWidth=".7" />
          <circle cy="23" r="2" fill="#33464d" />
          <circle cx={side * 20} cy="11" r="10" fill="#343c36" stroke="#8c9982" strokeWidth=".8" />
          <circle cx="-2" cy="-2" r="1.5" fill="#a1adb1" opacity=".35" />
        </g>)}
        <circle cx="146" cy="184" r="1.4" fill="#7e8975" />
      </>}
    </> : <>
      <path d="M145 139C112 55 218 6 290 66L319 127" stroke={paint('strap')} strokeWidth="27" />
      <path d="M147 137C125 72 218 29 273 75L301 130" stroke="#afbaae" strokeWidth="4" />
      <path d="M210 105 201 51 230 44 257 104" fill="#e9ede5" /><path d="m218 96-12-43 17-5 20 46" fill="#fafbf5" />
      <path d="M109 121c38-33 153-54 218-12 20 13 25 44 11 64l-44 44c-11 11-31 17-49 15l-112-20c-23-5-40-23-40-47 0-18 4-32 16-44Z" fill="#687369" />
      <path d="M94 128c18-28 146-43 207-25 22 6 35 23 34 46l-4 35c-2 26-24 45-50 46l-133-9c-35-3-59-18-62-44-2-18-1-35 8-49Z" fill={paint('visor')} />
      <path d="M105 132c37-24 130-28 181-18" stroke="white" strokeWidth="3" strokeLinecap="round" opacity=".9" />
      {variant === 'quest2' && <>
        <ellipse cx="105" cy="144" rx="7" ry="8" fill="#414a41" /><ellipse cx="310" cy="132" rx="6" ry="7" fill="#414a41" />
        <ellipse cx="102" cy="194" rx="6" ry="7" fill="#414a41" /><ellipse cx="304" cy="203" rx="6" ry="7" fill="#414a41" />
      </>}
      <path d="M185 163c7-12 20-12 27 0 8 12 21 12 27 0" stroke="#a7b0a7" strokeWidth="3.5" strokeLinecap="round" />
      <rect x="145" y="218" width="34" height="5" rx="2.5" fill="#7f8b7e" />
    </>}
  </svg>;
}
