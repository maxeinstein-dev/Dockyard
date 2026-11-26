import { zoneCity } from "../clock/zones";

/* The search of the Relógio mundial's settings (achado do autor,
   2026-11-05): a city typed finds its zone, without scrolling the list of
   every zone. Offline: the zones' own names, and the Portuguese names of
   the cities most looked for. */

/** How many zones a search shows at most. */
export const MAX_FOUND = 8;

/** Portuguese names (and other common ones) of zone cities, by zone: what
 * a search compares besides the zone's own name. */
const OTHER_NAMES: Readonly<Record<string, readonly string[]>> = {
  "Europe/Lisbon": ["Lisboa", "Portugal"],
  "Europe/London": ["Londres", "Inglaterra", "Reino Unido"],
  "Europe/Madrid": ["Madri", "Espanha"],
  "Europe/Paris": ["França"],
  "Europe/Rome": ["Roma", "Itália"],
  "Europe/Berlin": ["Berlim", "Alemanha"],
  "Europe/Amsterdam": ["Amsterdã", "Holanda", "Países Baixos"],
  "Europe/Brussels": ["Bruxelas", "Bélgica"],
  "Europe/Vienna": ["Viena", "Áustria"],
  "Europe/Zurich": ["Zurique", "Suíça"],
  "Europe/Stockholm": ["Estocolmo", "Suécia"],
  "Europe/Warsaw": ["Varsóvia", "Polônia"],
  "Europe/Athens": ["Atenas", "Grécia"],
  "Europe/Istanbul": ["Istambul", "Turquia"],
  "Europe/Moscow": ["Moscou", "Rússia"],
  "Europe/Dublin": ["Irlanda"],
  "Europe/Kyiv": ["Kiev", "Ucrânia"],
  "America/New_York": ["Nova York", "Nova Iorque", "Estados Unidos"],
  "America/Los_Angeles": ["Califórnia", "São Francisco"],
  "America/Chicago": ["Texas"],
  "America/Mexico_City": ["Cidade do México", "México"],
  "America/Toronto": ["Canadá"],
  "America/Havana": ["Cuba"],
  "America/Bogota": ["Bogotá", "Colômbia"],
  "America/Lima": ["Peru"],
  "America/Santiago": ["Chile"],
  "America/Argentina/Buenos_Aires": ["Argentina"],
  "America/Montevideo": ["Montevidéu", "Uruguai"],
  "America/Asuncion": ["Assunção", "Paraguai"],
  "America/Caracas": ["Venezuela"],
  "America/La_Paz": ["Bolívia"],
  "America/Sao_Paulo": [
    "São Paulo",
    "Brasília",
    "Rio de Janeiro",
    "Belo Horizonte",
    "Curitiba",
    "Porto Alegre",
    "Brasil",
  ],
  "America/Manaus": ["Amazonas"],
  "America/Recife": ["Pernambuco"],
  "America/Bahia": ["Salvador"],
  "America/Belem": ["Belém", "Pará"],
  "America/Cuiaba": ["Cuiabá", "Mato Grosso"],
  "America/Noronha": ["Fernando de Noronha"],
  "America/Rio_Branco": ["Acre"],
  "Asia/Tokyo": ["Tóquio", "Japão"],
  "Asia/Shanghai": ["Xangai", "Pequim", "China"],
  "Asia/Singapore": ["Cingapura", "Singapura"],
  "Asia/Seoul": ["Seul", "Coreia do Sul"],
  "Asia/Kolkata": ["Índia", "Nova Délhi", "Mumbai", "Calcutá"],
  "Asia/Dubai": ["Emirados Árabes"],
  "Asia/Jerusalem": ["Jerusalém", "Israel"],
  "Asia/Bangkok": ["Bangcoc", "Tailândia"],
  "Asia/Jakarta": ["Jacarta", "Indonésia"],
  "Asia/Manila": ["Filipinas"],
  "Asia/Taipei": ["Taiwan"],
  "Africa/Cairo": ["Egito"],
  "Africa/Johannesburg": ["Joanesburgo", "África do Sul"],
  "Africa/Luanda": ["Angola"],
  "Africa/Maputo": ["Moçambique"],
  "Africa/Lagos": ["Nigéria"],
  "Africa/Nairobi": ["Quênia"],
  "Australia/Sydney": ["Austrália"],
  "Pacific/Auckland": ["Nova Zelândia"],
};

/** Lower case, without accents: "Tóquio" finds "toquio". */
function folded(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().trim();
}

/** The names a search compares for `zone`: its city, its whole IANA name
 * and the other names it is known by. */
function namesOf(zone: string): string[] {
  return [
    zoneCity(zone),
    zone.replace(/_/g, " "),
    ...(OTHER_NAMES[zone] ?? []),
  ];
}

/**
 * The zones of `zones` whose names hold every word of `query`, those with a
 * name starting with it first, at most `MAX_FOUND`. An empty query finds
 * nothing.
 */
export function findZones(query: string, zones: readonly string[]): string[] {
  const words = folded(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const start = words.join(" ");
  const found: { zone: string; starts: boolean }[] = [];
  for (const zone of zones) {
    const names = namesOf(zone).map(folded);
    if (!names.some((name) => words.every((word) => name.includes(word)))) {
      continue;
    }
    found.push({ zone, starts: names.some((name) => name.startsWith(start)) });
  }
  return found
    .sort((a, b) => Number(b.starts) - Number(a.starts))
    .slice(0, MAX_FOUND)
    .map(({ zone }) => zone);
}

/** How a found zone is shown: the other name the search asked for, with
 * the zone's city after it ("Porto Alegre · Sao Paulo"), else the city. */
export function foundName(query: string, zone: string): string {
  const city = zoneCity(zone);
  const wanted = folded(query);
  const other = (OTHER_NAMES[zone] ?? []).find((name) =>
    folded(name).includes(wanted),
  );
  return other !== undefined && folded(other) !== folded(city)
    ? `${other} · ${city}`
    : city;
}
