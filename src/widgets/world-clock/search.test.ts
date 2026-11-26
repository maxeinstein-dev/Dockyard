import { describe, expect, it } from "vitest";
import { findZones, foundName, MAX_FOUND } from "./search";

const ZONES = [
  "America/New_York",
  "America/Sao_Paulo",
  "Asia/Tokyo",
  "Europe/Lisbon",
  "Europe/London",
];

describe("the search of the Relógio mundial", () => {
  it("finds a zone by its city, in any case and without accents", () => {
    expect(findZones("tokyo", ZONES)).toEqual(["Asia/Tokyo"]);
    expect(findZones("SAO PAULO", ZONES)).toEqual(["America/Sao_Paulo"]);
  });

  it("finds a zone by its Portuguese name or another city in it", () => {
    expect(findZones("Tóquio", ZONES)).toEqual(["Asia/Tokyo"]);
    expect(findZones("lisboa", ZONES)).toEqual(["Europe/Lisbon"]);
    expect(findZones("porto alegre", ZONES)).toEqual(["America/Sao_Paulo"]);
  });

  it("puts the names that start with the search first", () => {
    // "lon" starts London and is inside no other name here.
    expect(findZones("lon", ZONES)[0]).toBe("Europe/London");
  });

  it("finds nothing for an empty search, and at most a few zones", () => {
    expect(findZones("   ", ZONES)).toEqual([]);
    const many = Array.from({ length: 20 }, (_, at) => `Etc/Zone_${at}`);
    expect(findZones("zone", many)).toHaveLength(MAX_FOUND);
  });

  it("shows the name searched for beside the zone's city", () => {
    expect(foundName("porto", "America/Sao_Paulo")).toBe(
      "Porto Alegre · Sao Paulo",
    );
    expect(foundName("tokyo", "Asia/Tokyo")).toBe("Tokyo");
  });
});
