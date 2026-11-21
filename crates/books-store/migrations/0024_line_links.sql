ALTER TABLE line ADD COLUMN person_id INTEGER REFERENCES person (person_id);
ALTER TABLE line ADD COLUMN department_id INTEGER REFERENCES tag (tag_id);
ALTER TABLE line ADD COLUMN location_id INTEGER REFERENCES tag (tag_id);
ALTER TABLE line ADD COLUMN memo TEXT;
CREATE INDEX line_by_person ON line (person_id);
CREATE INDEX line_by_department ON line (department_id);
CREATE INDEX line_by_location ON line (location_id);
