-- Create additional schemas
CREATE SCHEMA IF NOT EXISTS clinical;
CREATE SCHEMA IF NOT EXISTS billing;
CREATE SCHEMA IF NOT EXISTS admin;
CREATE SCHEMA IF NOT EXISTS private;

-- Create functions in private schema for default values and other utilities

-- Function to generate a random appointment ID
CREATE OR REPLACE FUNCTION private.generate_appointment_id()
RETURNS TEXT AS $$
BEGIN
    RETURN 'APT-' || to_char(current_date, 'YYYYMMDD') || '-' || LPAD(FLOOR(random() * 10000)::TEXT, 4, '0');
END;
$$ LANGUAGE plpgsql;

-- Function to update timestamp
CREATE OR REPLACE FUNCTION private.update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function to generate invoice number
CREATE OR REPLACE FUNCTION private.generate_invoice_number()
RETURNS TEXT AS $$
DECLARE
    year_prefix TEXT;
    next_num INTEGER;
BEGIN
    year_prefix := to_char(current_date, 'YYYY');

    SELECT COALESCE(MAX(SUBSTRING(invoice_number FROM '[0-9]+$')::INTEGER), 0) + 1
    INTO next_num
    FROM billing.invoices
    WHERE invoice_number LIKE year_prefix || '-%';

    RETURN year_prefix || '-' || LPAD(next_num::TEXT, 6, '0');
END;
$$ LANGUAGE plpgsql;

------------------------------
-- ADMIN SCHEMA TABLES
------------------------------

-- Table for departments
CREATE TABLE admin.departments (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    head_doctor_id INTEGER,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_departments_timestamp
BEFORE UPDATE ON admin.departments
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE admin.departments ENABLE ROW LEVEL SECURITY;

-- RLS policies for departments
CREATE POLICY departments_view_policy ON admin.departments
    FOR SELECT
    USING (auth.role() = 'authenticated');

CREATE POLICY departments_insert_policy ON admin.departments
    FOR INSERT
    WITH CHECK (auth.role() = 'admin');

CREATE POLICY departments_update_policy ON admin.departments
    FOR UPDATE
    USING (auth.role() = 'admin');

CREATE POLICY departments_delete_policy ON admin.departments
    FOR DELETE
    USING (auth.role() = 'admin');

------------------------------
-- PUBLIC SCHEMA TABLES
------------------------------

-- Table for patients
CREATE TABLE public.patients (
    id SERIAL PRIMARY KEY,
    medical_record_number VARCHAR(20) UNIQUE NOT NULL,
    first_name VARCHAR(50) NOT NULL,
    last_name VARCHAR(50) NOT NULL,
    date_of_birth DATE NOT NULL,
    gender VARCHAR(20) CHECK (gender IN ('Male', 'Female', 'Non-binary', 'Other', 'Prefer not to say')),
    phone_number VARCHAR(20),
    email VARCHAR(100),
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_patients_timestamp
BEFORE UPDATE ON public.patients
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE public.patients ENABLE ROW LEVEL SECURITY;

-- RLS policies for patients
CREATE POLICY patients_view_policy ON public.patients
    FOR SELECT
    USING (
        auth.role() = 'authenticated' AND
        (
            auth.role() IN ('admin', 'doctor', 'nurse', 'receptionist', 'billing') OR
            created_by = auth.uid()
        )
    );

CREATE POLICY patients_insert_policy ON public.patients
    FOR INSERT
    WITH CHECK (
        auth.role() IN ('admin', 'receptionist') OR
        created_by = auth.uid()
    );

CREATE POLICY patients_update_policy ON public.patients
    FOR UPDATE
    USING (
        auth.role() IN ('admin', 'doctor', 'nurse', 'receptionist') OR
        created_by = auth.uid()
    );

CREATE POLICY patients_delete_policy ON public.patients
    FOR DELETE
    USING (auth.role() = 'admin');

-- Table for doctors
CREATE TABLE public.doctors (
    id SERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES auth.users(id),
    license_number VARCHAR(50) UNIQUE NOT NULL,
    specialization VARCHAR(100) NOT NULL,
    department_id INTEGER REFERENCES admin.departments(id),
    consultation_fee DECIMAL(10,2) CHECK (consultation_fee >= 0),
    available_for_appointments BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_doctors_timestamp
BEFORE UPDATE ON public.doctors
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE public.doctors ENABLE ROW LEVEL SECURITY;

-- RLS policies for doctors
CREATE POLICY doctors_view_policy ON public.doctors
    FOR SELECT
    USING (auth.role() = 'authenticated');

CREATE POLICY doctors_insert_policy ON public.doctors
    FOR INSERT
    WITH CHECK (
        auth.role() = 'admin' OR
        user_id = auth.uid()
    );

CREATE POLICY doctors_update_policy ON public.doctors
    FOR UPDATE
    USING (
        auth.role() = 'admin' OR
        user_id = auth.uid()
    );

CREATE POLICY doctors_delete_policy ON public.doctors
    FOR DELETE
    USING (auth.role() = 'admin');

-- Add the foreign key constraint to departments for head_doctor
ALTER TABLE admin.departments
ADD CONSTRAINT fk_head_doctor FOREIGN KEY (head_doctor_id) REFERENCES public.doctors(id);

------------------------------
-- CLINICAL SCHEMA TABLES
------------------------------

-- Table for appointments
CREATE TABLE clinical.appointments (
    id SERIAL PRIMARY KEY,
    appointment_id TEXT NOT NULL UNIQUE DEFAULT private.generate_appointment_id(),
    patient_id INTEGER NOT NULL REFERENCES public.patients(id),
    doctor_id INTEGER NOT NULL REFERENCES public.doctors(id),
    appointment_date DATE NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'scheduled' CHECK (status IN ('scheduled', 'confirmed', 'completed', 'cancelled', 'no-show')),
    reason_for_visit TEXT,
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT appointment_times CHECK (start_time < end_time)
);

CREATE TRIGGER update_appointments_timestamp
BEFORE UPDATE ON clinical.appointments
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE clinical.appointments ENABLE ROW LEVEL SECURITY;

-- RLS policies for appointments
CREATE POLICY appointments_view_policy ON clinical.appointments
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'doctor', 'nurse', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        ) OR
        EXISTS (
            SELECT 1 FROM public.doctors d
            WHERE d.id = doctor_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY appointments_insert_policy ON clinical.appointments
    FOR INSERT
    WITH CHECK (
        auth.role() IN ('admin', 'doctor', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY appointments_update_policy ON clinical.appointments
    FOR UPDATE
    USING (
        auth.role() IN ('admin', 'doctor', 'nurse', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.doctors d
            WHERE d.id = doctor_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY appointments_delete_policy ON clinical.appointments
    FOR DELETE
    USING (auth.role() IN ('admin', 'receptionist'));

-- Table for medical records
CREATE TABLE clinical.medical_records (
    id SERIAL PRIMARY KEY,
    patient_id INTEGER NOT NULL REFERENCES public.patients(id),
    doctor_id INTEGER NOT NULL REFERENCES public.doctors(id),
    appointment_id INTEGER REFERENCES clinical.appointments(id),
    diagnosis TEXT,
    treatment_plan TEXT,
    notes TEXT,
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_medical_records_timestamp
BEFORE UPDATE ON clinical.medical_records
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE clinical.medical_records ENABLE ROW LEVEL SECURITY;

-- RLS policies for medical records
CREATE POLICY medical_records_view_policy ON clinical.medical_records
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'doctor', 'nurse') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        ) OR
        EXISTS (
            SELECT 1 FROM public.doctors d
            WHERE d.id = doctor_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY medical_records_insert_policy ON clinical.medical_records
    FOR INSERT
    WITH CHECK (
        auth.role() IN ('admin', 'doctor', 'nurse') OR
        EXISTS (
            SELECT 1 FROM public.doctors d
            WHERE d.id = doctor_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY medical_records_update_policy ON clinical.medical_records
    FOR UPDATE
    USING (
        auth.role() IN ('admin', 'doctor') OR
        EXISTS (
            SELECT 1 FROM public.doctors d
            WHERE d.id = doctor_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY medical_records_delete_policy ON clinical.medical_records
    FOR DELETE
    USING (auth.role() = 'admin');

-- Table for prescriptions
CREATE TABLE clinical.prescriptions (
    id SERIAL PRIMARY KEY,
    medical_record_id INTEGER NOT NULL REFERENCES clinical.medical_records(id),
    medication_name VARCHAR(100) NOT NULL,
    dosage VARCHAR(50) NOT NULL,
    frequency VARCHAR(50) NOT NULL,
    duration INTEGER NOT NULL, -- in days
    issued_date DATE NOT NULL DEFAULT CURRENT_DATE,
    expiry_date DATE NOT NULL,
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_expiry CHECK (expiry_date > issued_date)
);

CREATE TRIGGER update_prescriptions_timestamp
BEFORE UPDATE ON clinical.prescriptions
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE clinical.prescriptions ENABLE ROW LEVEL SECURITY;

-- RLS policies for prescriptions
CREATE POLICY prescriptions_view_policy ON clinical.prescriptions
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'doctor', 'nurse', 'pharmacist') OR
        EXISTS (
            SELECT 1 FROM clinical.medical_records mr
            JOIN public.patients p ON mr.patient_id = p.id
            WHERE mr.id = medical_record_id AND p.created_by = auth.uid()
        ) OR
        EXISTS (
            SELECT 1 FROM clinical.medical_records mr
            JOIN public.doctors d ON mr.doctor_id = d.id
            WHERE mr.id = medical_record_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY prescriptions_insert_policy ON clinical.prescriptions
    FOR INSERT
    WITH CHECK (
        auth.role() IN ('admin', 'doctor') OR
        EXISTS (
            SELECT 1 FROM clinical.medical_records mr
            JOIN public.doctors d ON mr.doctor_id = d.id
            WHERE mr.id = medical_record_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY prescriptions_update_policy ON clinical.prescriptions
    FOR UPDATE
    USING (
        auth.role() IN ('admin', 'doctor') OR
        EXISTS (
            SELECT 1 FROM clinical.medical_records mr
            JOIN public.doctors d ON mr.doctor_id = d.id
            WHERE mr.id = medical_record_id AND d.user_id = auth.uid()
        )
    );

CREATE POLICY prescriptions_delete_policy ON clinical.prescriptions
    FOR DELETE
    USING (auth.role() = 'admin');

------------------------------
-- BILLING SCHEMA TABLES
------------------------------

-- Table for services
CREATE TABLE billing.services (
    id SERIAL PRIMARY KEY,
    service_code VARCHAR(20) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    category VARCHAR(50) NOT NULL,
    base_cost DECIMAL(10,2) NOT NULL CHECK (base_cost >= 0),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_services_timestamp
BEFORE UPDATE ON billing.services
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.services ENABLE ROW LEVEL SECURITY;

-- RLS policies for services
CREATE POLICY services_view_policy ON billing.services
    FOR SELECT
    USING (auth.role() = 'authenticated');

CREATE POLICY services_insert_policy ON billing.services
    FOR INSERT
    WITH CHECK (auth.role() IN ('admin', 'billing'));

CREATE POLICY services_update_policy ON billing.services
    FOR UPDATE
    USING (auth.role() IN ('admin', 'billing'));

CREATE POLICY services_delete_policy ON billing.services
    FOR DELETE
    USING (auth.role() = 'admin');

-- Table for insurance providers
CREATE TABLE billing.insurance_providers (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    contact_person VARCHAR(100),
    phone_number VARCHAR(20),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_insurance_providers_timestamp
BEFORE UPDATE ON billing.insurance_providers
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.insurance_providers ENABLE ROW LEVEL SECURITY;

-- RLS policies for insurance providers
CREATE POLICY insurance_providers_view_policy ON billing.insurance_providers
    FOR SELECT
    USING (auth.role() = 'authenticated');

CREATE POLICY insurance_providers_insert_policy ON billing.insurance_providers
    FOR INSERT
    WITH CHECK (auth.role() IN ('admin', 'billing'));

CREATE POLICY insurance_providers_update_policy ON billing.insurance_providers
    FOR UPDATE
    USING (auth.role() IN ('admin', 'billing'));

CREATE POLICY insurance_providers_delete_policy ON billing.insurance_providers
    FOR DELETE
    USING (auth.role() = 'admin');

-- Table for patient insurance
CREATE TABLE billing.patient_insurance (
    id SERIAL PRIMARY KEY,
    patient_id INTEGER NOT NULL REFERENCES public.patients(id),
    insurance_provider_id INTEGER NOT NULL REFERENCES billing.insurance_providers(id),
    policy_number VARCHAR(50) NOT NULL,
    coverage_start_date DATE NOT NULL,
    coverage_end_date DATE,
    is_primary BOOLEAN NOT NULL DEFAULT TRUE,
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_coverage_dates CHECK (coverage_end_date IS NULL OR coverage_end_date >= coverage_start_date)
);

CREATE TRIGGER update_patient_insurance_timestamp
BEFORE UPDATE ON billing.patient_insurance
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.patient_insurance ENABLE ROW LEVEL SECURITY;

-- RLS policies for patient insurance
CREATE POLICY patient_insurance_view_policy ON billing.patient_insurance
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY patient_insurance_insert_policy ON billing.patient_insurance
    FOR INSERT
    WITH CHECK (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY patient_insurance_update_policy ON billing.patient_insurance
    FOR UPDATE
    USING (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY patient_insurance_delete_policy ON billing.patient_insurance
    FOR DELETE
    USING (auth.role() IN ('admin', 'billing'));

-- Table for invoices
CREATE TABLE billing.invoices (
    id SERIAL PRIMARY KEY,
    invoice_number TEXT UNIQUE NOT NULL DEFAULT private.generate_invoice_number(),
    patient_id INTEGER NOT NULL REFERENCES public.patients(id),
    appointment_id INTEGER REFERENCES clinical.appointments(id),
    issued_date DATE NOT NULL DEFAULT CURRENT_DATE,
    due_date DATE NOT NULL,
    total_amount DECIMAL(10,2) NOT NULL CHECK (total_amount >= 0),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'paid', 'partially_paid', 'overdue', 'cancelled')),
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_due_date CHECK (due_date >= issued_date)
);

CREATE TRIGGER update_invoices_timestamp
BEFORE UPDATE ON billing.invoices
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.invoices ENABLE ROW LEVEL SECURITY;

-- RLS policies for invoices
CREATE POLICY invoices_view_policy ON billing.invoices
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM public.patients p
            WHERE p.id = patient_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY invoices_insert_policy ON billing.invoices
    FOR INSERT
    WITH CHECK (auth.role() IN ('admin', 'billing'));

CREATE POLICY invoices_update_policy ON billing.invoices
    FOR UPDATE
    USING (auth.role() IN ('admin', 'billing'));

CREATE POLICY invoices_delete_policy ON billing.invoices
    FOR DELETE
    USING (auth.role() = 'admin');

-- Table for invoice line items
CREATE TABLE billing.invoice_items (
    id SERIAL PRIMARY KEY,
    invoice_id INTEGER NOT NULL REFERENCES billing.invoices(id),
    service_id INTEGER NOT NULL REFERENCES billing.services(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(10,2) NOT NULL CHECK (unit_price >= 0),
    line_total DECIMAL(10,2) NOT NULL CHECK (line_total >= 0),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_invoice_items_timestamp
BEFORE UPDATE ON billing.invoice_items
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.invoice_items ENABLE ROW LEVEL SECURITY;

-- RLS policies for invoice items
CREATE POLICY invoice_items_view_policy ON billing.invoice_items
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM billing.invoices i
            JOIN public.patients p ON i.patient_id = p.id
            WHERE i.id = invoice_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY invoice_items_insert_policy ON billing.invoice_items
    FOR INSERT
    WITH CHECK (auth.role() IN ('admin', 'billing'));

CREATE POLICY invoice_items_update_policy ON billing.invoice_items
    FOR UPDATE
    USING (auth.role() IN ('admin', 'billing'));

CREATE POLICY invoice_items_delete_policy ON billing.invoice_items
    FOR DELETE
    USING (auth.role() IN ('admin', 'billing'));

-- Table for payments
CREATE TABLE billing.payments (
    id SERIAL PRIMARY KEY,
    invoice_id INTEGER NOT NULL REFERENCES billing.invoices(id),
    payment_date DATE NOT NULL DEFAULT CURRENT_DATE,
    amount_paid DECIMAL(10,2) NOT NULL CHECK (amount_paid > 0),
    payment_method VARCHAR(50) NOT NULL CHECK (payment_method IN ('cash', 'credit_card', 'debit_card', 'check', 'bank_transfer', 'insurance')),
    transaction_reference VARCHAR(100),
    created_by UUID NOT NULL REFERENCES auth.users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_payments_timestamp
BEFORE UPDATE ON billing.payments
FOR EACH ROW EXECUTE FUNCTION private.update_timestamp();

ALTER TABLE billing.payments ENABLE ROW LEVEL SECURITY;

-- RLS policies for payments
CREATE POLICY payments_view_policy ON billing.payments
    FOR SELECT
    USING (
        auth.role() IN ('admin', 'billing', 'receptionist') OR
        EXISTS (
            SELECT 1 FROM billing.invoices i
            JOIN public.patients p ON i.patient_id = p.id
            WHERE i.id = invoice_id AND p.created_by = auth.uid()
        )
    );

CREATE POLICY payments_insert_policy ON billing.payments
    FOR INSERT
    WITH CHECK (auth.role() IN ('admin', 'billing', 'receptionist'));

CREATE POLICY payments_update_policy ON billing.payments
    FOR UPDATE
    USING (auth.role() IN ('admin', 'billing'));

CREATE POLICY payments_delete_policy ON billing.payments
    FOR DELETE
    USING (auth.role() = 'admin');

-- Create a view to see outstanding balances
CREATE VIEW billing.outstanding_balances AS
SELECT
    i.id AS invoice_id,
    i.invoice_number,
    p.id AS patient_id,
    p.first_name || ' ' || p.last_name AS patient_name,
    i.total_amount,
    COALESCE(SUM(py.amount_paid), 0) AS amount_paid,
    (i.total_amount - COALESCE(SUM(py.amount_paid), 0)) AS balance_due,
    i.due_date,
    CASE
        WHEN i.due_date < CURRENT_DATE AND (i.total_amount - COALESCE(SUM(py.amount_paid), 0)) > 0 THEN 'overdue'
        WHEN (i.total_amount - COALESCE(SUM(py.amount_paid), 0)) = 0 THEN 'paid'
        WHEN (i.total_amount - COALESCE(SUM(py.amount_paid), 0)) > 0 THEN 'partial'
        ELSE 'unknown'
    END AS payment_status
FROM
    billing.invoices i
JOIN
    public.patients p ON i.patient_id = p.id
LEFT JOIN
    billing.payments py ON i.id = py.invoice_id
GROUP BY
    i.id, i.invoice_number, p.id, p.first_name, p.last_name, i.total_amount, i.due_date;
