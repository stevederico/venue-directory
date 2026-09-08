let meta = #{
    name: "fill-venue-metadata",
    description: "Parallel research then PATCH missing venue address, contact, screen, capacity",
    phases: [
        #{ title: "Research", detail: "one agent per venue batch" },
        #{ title: "Apply", detail: "PATCH verified fields" },
    ],
};

let patch_schema = #{
    "type": "object",
    "required": ["patches"],
    "properties": #{
        "patches": #{
            "type": "array",
            "maxItems": 40,
            "items": #{
                "type": "object",
                "required": ["slug"],
                "properties": #{
                    "slug": #{ "type": "string" },
                    "address": #{ "type": "string" },
                    "contact": #{ "type": "string" },
                    "screen": #{ "type": "string" },
                    "capacity": #{ "type": "string" },
                    "lat": #{ "type": "number" },
                    "lng": #{ "type": "number" },
                    "skip": #{ "type": "string" },
                },
            },
        },
    },
};

phase("Research");
let batches = [
    "convex framework shopify spacexai stripe",
    "gray-area-grand-theater the-hibernia public-works founders-inc corgi-cafe convene swedish-american-hall noisebridge",
    "dna-lounge sfjazz commonwealth-club openai doordash a16z house-of-ai vercel-sf-hq",
    "moscone-center chase-center san-francisco-museum-of-modern-art y-combinator hyatt-regency marines-memorial contemporary-jewish-museum salesforce-tower",
    "the-regency-ballroom the-midway spin-san-francisco okta carta intercom ideo tesla-san-francisco-van-ness",
];
let jobs = [];
let i = 0;
for b in batches {
    let p = "Research public metadata for these SF venue slugs. Use web_search. ";
    p += "Do not invent emails or phones. Do not geocode a name with no official street. ";
    p += "GET http://127.0.0.1:18792/api/venues/{slug} first. ";
    p += "Return patches with slug and any verified address, contact, screen, capacity, lat, lng. ";
    p += "If nothing verified, slug plus skip reason. Slugs: ";
    p += b;
    jobs.push(#{
        prompt: p,
        label: "batch-" + i.to_string(),
        capability_mode: "read-only",
        output_schema: patch_schema,
    });
    i += 1;
}
let results = parallel(jobs);
let all = [];
for r in results {
    if r != () && r.success && r.output.patches != () {
        for item in r.output.patches {
            all.push(item);
        }
    }
}
log(all.len().to_string() + " patch objects from research");
phase("Apply");
let apply_prompt = "Apply verified patches via PATCH http://127.0.0.1:18792/api/venues/{slug}. ";
apply_prompt += "Do not overwrite a longer existing contact; append with middle-dot. ";
apply_prompt += "Skip objects that have a skip field. JSON: ";
apply_prompt += json_encode(all);
let wr = agent(apply_prompt, #{ label: "apply-patches", capability_mode: "execute" });
let summary = "research objects=" + all.len().to_string();
if wr != () && wr.success {
    summary += " apply-ok";
}
complete(#{ summary: summary, n: all.len() });
